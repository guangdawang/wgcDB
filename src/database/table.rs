// src/database/table.rs
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use crate::core::condition::{Condition, check_condition};

#[derive(Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: BTreeMap<u64, Vec<String>>,
    pub next_id: u64,
    pub indexes: HashMap<String, BTreeMap<String, Vec<u64>>>, // key = 列名
    #[serde(default)]
    pub index_name_to_col: HashMap<String, String>,            // 索引名 -> 列名
}

impl Table {
    pub fn new(columns: Vec<String>) -> Self {
        Table {
            columns,
            rows: BTreeMap::new(),
            next_id: 1,
            indexes: HashMap::new(),
            index_name_to_col: HashMap::new(),
        }
    }

    pub fn insert_row(&mut self, values: Vec<String>) -> Result<u64, String> {
        if values.len() != self.columns.len() {
            return Err("Column count mismatch".into());
        }
        let id = self.next_id;
        self.next_id += 1;

        for (col_name, btree) in self.indexes.iter_mut() {
            if let Some(col_idx) = self.columns.iter().position(|c| c == col_name) {
                let val = &values[col_idx];
                btree.entry(val.clone()).or_default().push(id);
            }
        }
        self.rows.insert(id, values);
        Ok(id)
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
                if old_val == &new_val {
                    continue;
                }
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

    /// 构建索引，需要索引名和列名
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

    /// 删除索引，优先按索引名查找，失败则按列名尝试
    pub fn drop_index(&mut self, name: &str) {
        // 先按列名删除
        if self.indexes.remove(name).is_some() {
            self.index_name_to_col.retain(|_, col| col != name);
            return;
        }
        // 再尝试作为索引名删除
        if let Some(col) = self.index_name_to_col.remove(name) {
            self.indexes.remove(&col);
        }
    }

    pub fn scan_with_index(&self, cond: &Condition) -> Option<(Vec<(u64, &Vec<String>)>, usize)> {
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