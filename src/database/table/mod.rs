// src/database/table/mod.rs
mod insert;
mod index;
mod scan;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: BTreeMap<u64, Vec<String>>,
    pub next_id: u64,
    pub indexes: HashMap<String, BTreeMap<String, Vec<u64>>>,
    #[serde(default)]
    pub index_name_to_col: HashMap<String, String>,
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
}