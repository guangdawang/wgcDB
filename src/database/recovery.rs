// src/database/recovery.rs
use crate::database::wal;
use crate::Database;

impl Database {
    /// 从 WAL 恢复数据库（应在启动时调用）
    pub fn recover(&mut self) -> Result<(), String> {
        if let Some(wal_path) = &self.wal_path {
            let wal_path = wal_path.clone();
            wal::replay_wal(&wal_path, self)?;
            wal::clear(&wal_path)?;
        }
        Ok(())
    }
}