mod stats;
mod table;

pub use stats::Stats;
pub use table::Table;

use std::collections::HashMap;

pub struct Database {
    pub tables: HashMap<String, Table>,
    pub stats: Stats,
}

impl Database {
    pub fn new(threshold: u32) -> Self {
        Database {
            tables: HashMap::new(),
            stats: Stats::new(threshold),
        }
    }

    pub fn add_table(&mut self, name: &str, columns: Vec<String>) {
        self.tables.insert(name.to_string(), Table::new(columns));
    }
}