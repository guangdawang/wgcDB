// src/database/mod.rs
mod stats;
mod table;
pub mod wal;
pub mod transaction;
mod recovery;
mod write;

pub use stats::Stats;
pub use table::Table;
pub use transaction::Transaction;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use self::wal::{WalRecord, append_record};

#[derive(Serialize, Deserialize)]
pub struct Database {
    pub tables: HashMap<String, Table>,
    pub stats: Stats,
    #[serde(skip)]
    pub wal_path: Option<String>,
    #[serde(skip)]
    pub active_transaction: Option<Transaction>,
}

impl Database {
    pub fn new(threshold: u32) -> Self {
        Database {
            tables: HashMap::new(),
            stats: Stats::new(threshold),
            wal_path: None,
            active_transaction: None,
        }
    }

    pub fn set_wal_path(&mut self, path: &str) {
        self.wal_path = Some(path.to_string());
    }

    pub fn add_table(&mut self, name: &str, columns: Vec<String>) {
        self.tables.insert(name.to_string(), Table::new(columns));
    }

    pub fn load(path: &str) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("读取文件失败: {}", e))?;
        let (db, _) = bincode::serde::decode_from_slice(&data, bincode::config::standard())
            .map_err(|e| format!("二进制反序列化失败: {}", e))?;
        Ok(db)
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        let data = bincode::serde::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| format!("序列化失败: {}", e))?;
        std::fs::write(path, data).map_err(|e| format!("写入文件失败: {}", e))?;
        Ok(())
    }

    pub fn begin_transaction(&mut self) -> Result<(), String> {
        if self.active_transaction.is_some() {
            return Err("已有活跃事务".into());
        }
        self.active_transaction = Some(Transaction::new());
        Ok(())
    }

    pub fn commit_transaction(&mut self) -> Result<(), String> {
        let tx = self.active_transaction.take()
            .ok_or("没有活跃事务")?;
        if tx.is_empty() {
            return Ok(());
        }
        let wal_path = self.wal_path.clone()
            .ok_or("未设置 WAL 路径，无法提交事务")?;

        append_record(&wal_path, &WalRecord::Begin)?;
        for rec in &tx.records {
            append_record(&wal_path, rec)?;
        }
        append_record(&wal_path, &WalRecord::Commit)?;

        for rec in &tx.records {
            self.apply_wal_record(rec)?;
        }
        Ok(())
    }

    pub fn rollback_transaction(&mut self) -> Result<(), String> {
        let tx = self.active_transaction.take()
            .ok_or("没有活跃事务")?;
        if let Some(ref wal_path) = self.wal_path {
            if !tx.is_empty() {
                append_record(wal_path, &WalRecord::Rollback)?;
            }
        }
        Ok(())
    }
}