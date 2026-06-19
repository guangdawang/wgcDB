use serde::{Deserialize, Serialize};
use crate::core::condition::Condition;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WalRecord {
    Begin,
    Commit,
    Rollback,
    CreateTable { name: String, columns: Vec<String> },
    CreateIndex { table: String, index_name: String, column: String },
    DropIndex { table: String, index_name: String },
    Insert { table: String, id: u64, values: Vec<String> },
    Update { table: String, conditions: Vec<Condition>, set_col: String, set_val: String },
    Delete { table: String, conditions: Vec<Condition> },
}

pub fn append_record(path: &str, record: &WalRecord) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("打开 WAL 失败: {}", e))?;
    let line = serde_json::to_string(record).map_err(|e| format!("序列化 WAL 记录失败: {}", e))?;
    writeln!(file, "{}", line).map_err(|e| format!("写入 WAL 失败: {}", e))?;
    file.flush().map_err(|e| format!("刷新 WAL 失败: {}", e))?;
    Ok(())
}

pub fn read_records(path: &str) -> Result<Vec<WalRecord>, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("读取 WAL 失败: {}", e)),
    };
    let mut records = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let rec: WalRecord =
            serde_json::from_str(line).map_err(|e| format!("解析 WAL 记录失败: {}", e))?;
        records.push(rec);
    }
    Ok(records)
}

/// 按事务重放 WAL：只应用完整的事务（Begin … Commit）
pub fn replay_wal(path: &str, db: &mut crate::Database) -> Result<(), String> {
    let records = read_records(path)?;
    let mut transaction: Vec<WalRecord> = Vec::new();
    let mut in_transaction = false;

    for rec in records {
        match rec {
            WalRecord::Begin => {
                if in_transaction {
                    transaction.clear();
                }
                in_transaction = true;
                transaction.clear();
            }
            WalRecord::Commit => {
                if in_transaction {
                    for op in &transaction {
                        db.apply_wal_record(op)?;
                    }
                    in_transaction = false;
                    transaction.clear();
                }
            }
            WalRecord::Rollback => {
                in_transaction = false;
                transaction.clear();
            }
            other => {
                if in_transaction {
                    transaction.push(other);
                } else {
                    db.apply_wal_record(&other)?;
                }
            }
        }
    }
    Ok(())
}

pub fn clear(path: &str) -> Result<(), String> {
    std::fs::write(path, "").map_err(|e| format!("清空 WAL 失败: {}", e))?;
    Ok(())
}