use super::Table;

impl Table {
    pub fn insert_row(&mut self, values: Vec<String>) -> Result<u64, String> {
        if values.len() != self.columns.len() {
            return Err("Column count mismatch".into());
        }
        let id = self.next_id;
        self.next_id += 1;
        self.insert_row_with_id(id, values)?;
        Ok(id)
    }

    pub fn insert_row_with_id(&mut self, id: u64, values: Vec<String>) -> Result<(), String> {
        if values.len() != self.columns.len() {
            return Err("Column count mismatch".into());
        }
        if self.rows.contains_key(&id) {
            return Ok(());
        }
        if id >= self.next_id {
            self.next_id = id + 1;
        }
        for (col_name, btree) in self.indexes.iter_mut() {
            if let Some(col_idx) = self.columns.iter().position(|c| c == col_name) {
                let val = &values[col_idx];
                btree.entry(val.clone()).or_default().push(id);
            }
        }
        self.rows.insert(id, values);
        Ok(())
    }
}