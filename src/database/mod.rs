// src/database/mod.rs
mod stats;
mod table;
pub mod wal;
pub mod transaction;

pub use stats::Stats;
pub use table::Table;
pub use transaction::Transaction;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use self::wal::{WalRecord, append_record, replay_wal};
use crate::executor::ExecutionResult;

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
        let data = std::fs::read_to_string(path).map_err(|e| format!("读取文件失败: {}", e))?;
        let db: Database = serde_json::from_str(&data).map_err(|e| format!("JSON 解析失败: {}", e))?;
        Ok(db)
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        let data = serde_json::to_string_pretty(self).map_err(|e| format!("序列化失败: {}", e))?;
        std::fs::write(path, data).map_err(|e| format!("写入文件失败: {}", e))?;
        Ok(())
    }

    /// 从 WAL 恢复数据库（应在启动时调用）
    pub fn recover(&mut self) -> Result<(), String> {
        if let Some(wal_path) = &self.wal_path {
            let wal_path = wal_path.clone();
            replay_wal(&wal_path, self)?;
            wal::clear(&wal_path)?;
        }
        Ok(())
    }

    /// 开始显式事务
    pub fn begin_transaction(&mut self) -> Result<(), String> {
        if self.active_transaction.is_some() {
            return Err("已有活跃事务".into());
        }
        self.active_transaction = Some(Transaction::new());
        Ok(())
    }

    /// 提交事务：先写 WAL，再应用操作
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

    /// 回滚事务
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

    /// 处理写操作：事务中缓存，否则作为隐式事务立即执行
    pub fn handle_write(&mut self, record: WalRecord) -> Result<ExecutionResult, String> {
        if let Some(ref mut tx) = self.active_transaction {
            tx.records.push(record);
            Ok(ExecutionResult::Pending)
        } else {
            if let Some(ref wal_path) = self.wal_path {
                append_record(wal_path, &WalRecord::Begin)?;
                append_record(wal_path, &record)?;
                append_record(wal_path, &WalRecord::Commit)?;
            }
            self.apply_wal_record_to_result(&record)
        }
    }

    fn apply_wal_record_to_result(&mut self, record: &WalRecord) -> Result<ExecutionResult, String> {
        match record {
            WalRecord::Insert { table, values, .. } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                tbl.insert_row(values.clone())?;
                Ok(ExecutionResult::Insert { count: 1 })
            }
            WalRecord::Update { table, conditions, set_col, set_val } => {
                let count = self.tables.get_mut(table).ok_or("表不存在")?
                    .update_rows(conditions, set_col, set_val)?;
                Ok(ExecutionResult::Update { count })
            }
            WalRecord::Delete { table, conditions } => {
                let count = self.tables.get_mut(table).ok_or("表不存在")?
                    .delete_rows(conditions)?;
                Ok(ExecutionResult::Delete { count })
            }
            WalRecord::CreateTable { name, columns } => {
                if !self.tables.contains_key(name) {
                    self.add_table(name, columns.clone());
                }
                Ok(ExecutionResult::CreateTable)
            }
            WalRecord::CreateIndex { table, index_name, column } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                if !tbl.indexes.contains_key(column) {
                    tbl.build_index(index_name, column)?;
                }
                Ok(ExecutionResult::CreateIndex)
            }
            WalRecord::DropIndex { table, index_name } => {
                if let Some(tbl) = self.tables.get_mut(table) {
                    tbl.drop_index(index_name);
                }
                Ok(ExecutionResult::DropIndex)
            }
            _ => Err("不支持的操作".into()),
        }
    }

    pub fn apply_wal_record(&mut self, record: &WalRecord) -> Result<(), String> {
        match record {
            WalRecord::CreateTable { name, columns } => {
                if !self.tables.contains_key(name) {
                    self.add_table(name, columns.clone());
                }
            }
            WalRecord::CreateIndex { table, index_name, column } => {
                if let Some(tbl) = self.tables.get_mut(table) {
                    if !tbl.indexes.contains_key(column) {
                        tbl.build_index(index_name, column)?;
                    }
                }
            }
            WalRecord::DropIndex { table, index_name } => {
                if let Some(tbl) = self.tables.get_mut(table) {
                    tbl.drop_index(index_name);
                }
            }
            WalRecord::Insert { table, id, values } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                if *id == 0 {
                    tbl.insert_row(values.clone())?;
                } else {
                    tbl.insert_row_with_id(*id, values.clone())?;
                }
            }
            WalRecord::Update { table, conditions, set_col, set_val } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                tbl.update_rows(conditions, set_col, set_val)?;
            }
            WalRecord::Delete { table, conditions } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                tbl.delete_rows(conditions)?;
            }
            WalRecord::Begin | WalRecord::Commit | WalRecord::Rollback => {}
        }
        Ok(())
    }
}