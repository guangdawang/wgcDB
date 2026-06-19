use crate::core::condition::{Condition, check_condition};
use super::Table;

impl Table {
    pub fn scan_with_condition(&self, conditions: &[Condition]) -> Result<(Vec<(u64, &Vec<String>)>, usize), String> {
        let mut result = Vec::new();
        let scanned = self.rows.len();
        for (&id, row) in &self.rows {
            if conditions.iter().all(|c| check_condition(row, &self.columns, c).unwrap_or(false)) {
                result.push((id, row));
            }
        }
        Ok((result, scanned))
    }

    pub fn update_rows(&mut self, conditions: &[Condition], set_col: &str, set_val: &str) -> Result<usize, String> {
        let col_idx = self.columns.iter().position(|c| c == set_col)
            .ok_or("SET column not found")?;
        let target_ids: Vec<u64> = self.rows.iter()
            .filter(|(_, row)| conditions.iter().all(|c| check_condition(row, &self.columns, c).unwrap_or(false)))
            .map(|(id, _)| *id)
            .collect();
        let count = target_ids.len();
        for id in &target_ids {
            if let Some(row) = self.rows.get_mut(id) {
                let old_val = &row[col_idx];
                let new_val = set_val.to_string();
                if old_val == &new_val { continue; }
                if let Some(index) = self.indexes.get_mut(set_col) {
                    if let Some(vec) = index.get_mut(old_val) {
                        vec.retain(|&x| x != *id);
                        if vec.is_empty() { index.remove(old_val); }
                    }
                    index.entry(new_val.clone()).or_default().push(*id);
                }
                row[col_idx] = new_val;
            }
        }
        Ok(count)
    }

    pub fn delete_rows(&mut self, conditions: &[Condition]) -> Result<usize, String> {
        let ids_to_delete: Vec<u64> = self.rows.iter()
            .filter(|(_, row)| conditions.iter().all(|c| check_condition(row, &self.columns, c).unwrap_or(false)))
            .map(|(id, _)| *id)
            .collect();
        let count = ids_to_delete.len();
        for id in &ids_to_delete {
            if let Some(row) = self.rows.remove(id) {
                for (col_name, btree) in self.indexes.iter_mut() {
                    if let Some(col_idx) = self.columns.iter().position(|c| c == col_name) {
                        let val = &row[col_idx];
                        if let Some(vec) = btree.get_mut(val) {
                            vec.retain(|&x| x != *id);
                            if vec.is_empty() { btree.remove(val); }
                        }
                    }
                }
            }
        }
        Ok(count)
    }
}