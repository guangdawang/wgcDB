use serde::{Deserialize, Serialize};
use crate::core::condition::Condition;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WalRecord {
    CreateTable { name: String, columns: Vec<String> },
    CreateIndex { table: String, column: String },
    DropIndex { table: String, column: String },
    Insert { table: String, values: Vec<String> },
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

pub fn clear(path: &str) -> Result<(), String> {
    std::fs::write(path, "").map_err(|e| format!("清空 WAL 失败: {}", e))?;
    Ok(())
}