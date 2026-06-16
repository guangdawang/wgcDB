use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Stats {
    counts: HashMap<String, u32>,
    threshold: u32,
}

impl Stats {
    pub fn new(threshold: u32) -> Self {
        Stats {
            counts: HashMap::new(),
            threshold,
        }
    }

    /// 记录一次查询，返回是否需要建立索引的 (table_name, column_name)
    pub fn record_and_check(
        &mut self,
        table_name: &str,
        column_name: &str,
        has_index: bool,
    ) -> Option<(String, String)> {
        if has_index {
            return None; // 已有索引就不再提示
        }
        let key = format!("{}.{}", table_name, column_name);
        let count = self.counts.entry(key.clone()).or_insert(0);
        *count += 1;
        if *count >= self.threshold {
            Some((table_name.to_string(), column_name.to_string()))
        } else {
            None
        }
    }
}