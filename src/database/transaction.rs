// src/database/transaction.rs
use crate::database::wal::WalRecord;

#[derive(Debug, Default)]
pub struct Transaction {
    /// 事务中累积的修改操作记录（不含 SELECT）
    pub records: Vec<WalRecord>,
}

impl Transaction {
    pub fn new() -> Self {
        Transaction { records: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}