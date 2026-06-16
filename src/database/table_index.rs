use super::Table;
use crate::executor::condition::Condition;
use std::collections::BTreeMap;

impl Table {
    pub fn build_index(&mut self, column_name: &str) -> Result<(), String> {
        let col_idx = self.columns.iter().position(|c| c == column_name)
            .ok_or("Column not found")?;
        let mut btree: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for (&id, row) in &self.rows {
            let val = &row[col_idx];
            btree.entry(val.clone()).or_default().push(id);
        }
        self.indexes.insert(column_name.to_string(), btree);
        Ok(())
    }

    pub fn drop_index(&mut self, column_name: &str) {
        self.indexes.remove(column_name);
    }

    pub fn scan_with_index(&self, cond: &Condition) -> Option<(Vec<(u64, &Vec<String>)>, usize)> {
        if cond.op != "=" { return None; }
        let index = self.indexes.get(&cond.col)?;
        // 修复：即使索引中不存在该值，也返回空结果并标记为“使用了索引”
        let ids = match index.get(&cond.val) {
            Some(ids) => ids,
            None => return Some((vec![], 0)),
        };
        let mut rows = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(row) = self.rows.get(id) {
                rows.push((*id, row));
            }
        }
        let len = rows.len();
        Some((rows, len))
    }
}