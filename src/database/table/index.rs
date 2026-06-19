use std::collections::BTreeMap;
use super::Table;

impl Table {
    pub fn build_index(&mut self, index_name: &str, column_name: &str) -> Result<(), String> {
        let col_idx = self.columns.iter().position(|c| c == column_name)
            .ok_or("Column not found")?;
        let mut btree: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for (&id, row) in &self.rows {
            let val = &row[col_idx];
            btree.entry(val.clone()).or_default().push(id);
        }
        self.indexes.insert(column_name.to_string(), btree);
        self.index_name_to_col.insert(index_name.to_string(), column_name.to_string());
        Ok(())
    }

    pub fn drop_index(&mut self, name: &str) {
        if self.indexes.remove(name).is_some() {
            self.index_name_to_col.retain(|_, col| col != name);
            return;
        }
        if let Some(col) = self.index_name_to_col.remove(name) {
            self.indexes.remove(&col);
        }
    }

    pub fn scan_with_index(&self, cond: &crate::core::condition::Condition) -> Option<(Vec<(u64, &Vec<String>)>, usize)> {
        if cond.op != "=" { return None; }
        let index = self.indexes.get(&cond.col)?;
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