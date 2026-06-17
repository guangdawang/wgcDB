mod stats;
mod table;
mod table_index;
pub mod wal;            // 将 wal 作为 database 的子模块

pub use stats::Stats;
pub use table::Table;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use self::wal::WalRecord;   // 现在从自身子模块引用

#[derive(Serialize, Deserialize)]
pub struct Database {
    pub tables: HashMap<String, Table>,
    pub stats: Stats,
}

impl Database {
    pub fn new(threshold: u32) -> Self {
        Database {
            tables: HashMap::new(),
            stats: Stats::new(threshold),
        }
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

    pub fn apply_wal_record(&mut self, record: &WalRecord) -> Result<(), String> {
        match record {
            WalRecord::CreateTable { name, columns } => {
                if !self.tables.contains_key(name) {
                    self.add_table(name, columns.clone());
                }
            }
            WalRecord::CreateIndex { table, column } => {
                if let Some(tbl) = self.tables.get_mut(table) {
                    if !tbl.indexes.contains_key(column) {
                        tbl.build_index(column)?;
                    }
                }
            }
            WalRecord::DropIndex { table, column } => {
                if let Some(tbl) = self.tables.get_mut(table) {
                    tbl.drop_index(column);
                }
            }
            WalRecord::Insert { table, values } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                tbl.insert_row(values.clone())?;
            }
            WalRecord::Update { table, conditions, set_col, set_val } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                tbl.update_rows(conditions, set_col, set_val)?;
            }
            WalRecord::Delete { table, conditions } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                tbl.delete_rows(conditions)?;
            }
        }
        Ok(())
    }
}