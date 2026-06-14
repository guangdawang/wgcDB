use std::collections::{BTreeMap, HashMap};

pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub indexes: HashMap<String, BTreeMap<String, Vec<usize>>>,
}

impl Table {
    pub fn new(columns: Vec<String>) -> Self {
        Table {
            columns,
            rows: Vec::new(),
            indexes: HashMap::new(),
        }
    }

    pub fn insert_row(&mut self, values: Vec<String>) -> Result<(), String> {
        if values.len() != self.columns.len() {
            return Err("Column count mismatch".into());
        }
        self.indexes.clear(); // 插入新数据后让已有索引失效（简单策略）
        self.rows.push(values);
        Ok(())
    }

    // 构建指定列的 BTreeMap 索引
    pub fn build_index(&mut self, column_name: &str) -> Result<(), String> {
        let col_idx = self
            .columns
            .iter()
            .position(|c| c == column_name)
            .ok_or("Column not found")?;
        let mut btree: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (row_id, row) in self.rows.iter().enumerate() {
            let val = &row[col_idx];
            btree.entry(val.clone()).or_default().push(row_id);
        }
        self.indexes.insert(column_name.to_string(), btree);
        Ok(())
    }
}