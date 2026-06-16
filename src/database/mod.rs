mod stats;
mod table;

pub use stats::Stats;
pub use table::Table;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    /// 从文件加载数据库（不再需要 threshold 参数）
    pub fn load(path: &str) -> Result<Self, String> {
        let data =
            std::fs::read_to_string(path).map_err(|e| format!("读取文件失败: {}", e))?;
        let db: Database =
            serde_json::from_str(&data).map_err(|e| format!("JSON 解析失败: {}", e))?;
        Ok(db)
    }

    /// 保存数据库到文件
    pub fn save(&self, path: &str) -> Result<(), String> {
        let data =
            serde_json::to_string_pretty(self).map_err(|e| format!("序列化失败: {}", e))?;
        std::fs::write(path, data).map_err(|e| format!("写入文件失败: {}", e))?;
        Ok(())
    }
}