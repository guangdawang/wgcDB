// src/database/write.rs
use crate::database::wal::{WalRecord, append_record};
use crate::executor::ExecutionResult;
use crate::Database;

impl Database {
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
            WalRecord::Insert { .. } => {
                self.apply_wal_record(record)?;
                Ok(ExecutionResult::Insert { count: 1 })
            }
            WalRecord::Update { table, conditions, set_col, set_val } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                let count = tbl.update_rows(conditions, set_col, set_val)?;
                Ok(ExecutionResult::Update { count })
            }
            WalRecord::Delete { table, conditions } => {
                let tbl = self.tables.get_mut(table).ok_or("表不存在")?;
                let count = tbl.delete_rows(conditions)?;
                Ok(ExecutionResult::Delete { count })
            }
            WalRecord::CreateTable { .. } => {
                self.apply_wal_record(record)?;
                Ok(ExecutionResult::CreateTable)
            }
            WalRecord::CreateIndex { .. } => {
                self.apply_wal_record(record)?;
                Ok(ExecutionResult::CreateIndex)
            }
            WalRecord::DropIndex { .. } => {
                self.apply_wal_record(record)?;
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