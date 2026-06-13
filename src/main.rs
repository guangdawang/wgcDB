use sqlparser::ast::{
    BinaryOperator, Expr, Select, SelectItem, SetExpr, Statement, TableFactor,
    Value as SqlValue, ValueWithSpan,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::{BTreeMap, HashMap};

struct Table {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    indexes: HashMap<String, BTreeMap<String, Vec<usize>>>,
}

struct Database {
    tables: HashMap<String, Table>,
    stats: HashMap<String, u32>,
    threshold: u32,
}

impl Database {
    fn new(threshold: u32) -> Self {
        Database {
            tables: HashMap::new(),
            stats: HashMap::new(),
            threshold,
        }
    }

    fn add_table(&mut self, name: &str, columns: Vec<String>) {
        self.tables.insert(
            name.to_string(),
            Table {
                columns,
                rows: Vec::new(),
                indexes: HashMap::new(),
            },
        );
    }

    fn insert(&mut self, table_name: &str, values: Vec<String>) -> Result<(), String> {
        let table = self
            .tables
            .get_mut(table_name)
            .ok_or("Table not found")?;
        if values.len() != table.columns.len() {
            return Err("Column count mismatch".into());
        }
        table.indexes.clear();
        table.rows.push(values);
        Ok(())
    }

    fn query(&mut self, sql: &str) -> Result<(Vec<Vec<String>>, usize, bool), String> {
        let dialect = GenericDialect {};
        let ast = Parser::parse_sql(&dialect, sql).map_err(|e| format!("Parse error: {e}"))?;
        let mut results = Vec::new();
        let scanned_rows;
        let mut used_index = false;

        for statement in ast {
            match statement {
                Statement::Query(query) => {
                    let Select {
                        projection,
                        from,
                        selection,
                        ..
                    } = match *query.body {
                        SetExpr::Select(select) => *select, // 解引用 Box<Select>
                        _ => return Err("Only simple SELECT".into()),
                    };

                    if !matches!(projection[0], SelectItem::Wildcard(_)) {
                        return Err("Only SELECT * supported".into());
                    }
                    let table_name = match &from[0].relation {
                        TableFactor::Table { name, .. } => name.to_string(),
                        _ => return Err("Only simple table names".into()),
                    };
                    let table = self.tables.get(&table_name).ok_or("Table not found")?;

                    let (filter_col, filter_val) = if let Some(Expr::BinaryOp {
                        left,
                        op,
                        right,
                    }) = selection
                    {
                        if op != BinaryOperator::Eq {
                            // 直接比较，不用 *
                            return Err("Only = supported".into());
                        }
                        let col = if let Expr::Identifier(ident) = *left {
                            ident.value
                        } else {
                            return Err("Left side must be column".into());
                        };
                        let val = match *right {
                            Expr::Value(ValueWithSpan {
                                value: SqlValue::SingleQuotedString(s),
                                ..
                            }) => s,
                            Expr::Value(ValueWithSpan {
                                value: SqlValue::Number(n, _),
                                ..
                            }) => n,
                            _ => return Err("Right side must be literal".into()),
                        };
                        (col, val)
                    } else {
                        return Err("WHERE clause required".into());
                    };

                    let stat_key = format!("{table_name}.{filter_col}");
                    let hits = self.stats.entry(stat_key.clone()).or_insert(0);
                    *hits += 1;

                    if let Some(index) = table.indexes.get(&filter_col) {
                        used_index = true;
                        if let Some(row_ids) = index.get(&filter_val) {
                            scanned_rows = row_ids.len();
                            for &row_id in row_ids {
                                results.push(table.rows[row_id].clone());
                            }
                        } else {
                            scanned_rows = 0;
                        }
                    } else {
                        let col_idx = table
                            .columns
                            .iter()
                            .position(|c| *c == filter_col)
                            .ok_or("Column not found")?;
                        scanned_rows = table.rows.len();
                        for row in &table.rows {
                            if row[col_idx] == filter_val {
                                results.push(row.clone());
                            }
                        }
                    }

                    return Ok((results, scanned_rows, used_index));
                }
                _ => return Err("Only SELECT supported".into()),
            }
        }
        Err("No query executed".into())
    }

    fn auto_build_indexes(&mut self) {
        let mut to_build = Vec::new();
        for (stat_key, hits) in &self.stats {
            if *hits >= self.threshold {
                let parts: Vec<&str> = stat_key.splitn(2, '.').collect();
                if parts.len() == 2 {
                    let table_name = parts[0].to_string();
                    let col_name = parts[1].to_string();
                    if let Some(table) = self.tables.get(&table_name) {
                        if !table.indexes.contains_key(&col_name) {
                            to_build.push((table_name, col_name));
                        }
                    }
                }
            }
        }

        for (table_name, col_name) in to_build {
            if let Some(table) = self.tables.get_mut(&table_name) {
                let col_idx = match table.columns.iter().position(|c| *c == col_name) {
                    Some(idx) => idx,
                    None => continue,
                };
                let mut btree: BTreeMap<String, Vec<usize>> = BTreeMap::new();
                for (row_id, row) in table.rows.iter().enumerate() {
                    let val = &row[col_idx];
                    btree.entry(val.clone()).or_default().push(row_id);
                }
                table.indexes.insert(col_name.clone(), btree);
                println!(
                    "⚡ 自动创建索引: {}.{} (触发查询次数: {})",
                    table_name, col_name, self.threshold
                );
            }
        }
    }
}

fn main() {
    let mut db = Database::new(5);

    db.add_table("users", vec!["id".into(), "name".into(), "age".into()]);

    let names = ["Alice", "Bob", "Charlie", "Diana", "Eve"];
    for i in 0..1000 {
        let id = i.to_string();
        let name = names[i % names.len()].to_string();
        let age = (20 + (i % 30)).to_string();
        db.insert("users", vec![id, name, age]).unwrap();
    }
    println!("初始数据: 1000 行已插入\n");

    let sql = "SELECT * FROM users WHERE name = 'Alice'";

    for i in 1..=12 {
        match db.query(sql) {
            Ok((rows, scanned, used_idx)) => {
                println!(
                    "查询 #{:<2}: 结果 {} 行 | 扫描 {} 行 | {}",
                    i,
                    rows.len(),
                    scanned,
                    if used_idx { "📌 使用了索引" } else { "🔍 全表扫描" }
                );
            }
            Err(e) => println!("查询 #{i} 错误: {e}"),
        }
        db.auto_build_indexes();
    }
}