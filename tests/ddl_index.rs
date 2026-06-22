mod common;
use common::init_db;
use wgc_db::{execute_sql, ExecutionResult};

#[test]
fn create_index_and_query() {
    let mut db = init_db();
    execute_sql(&mut db, "CREATE INDEX idx_age ON users (age)").unwrap();
    let res = execute_sql(&mut db, "SELECT * FROM users WHERE age = 25").unwrap();
    match res {
        ExecutionResult::Select {
            rows,
            scanned,
            used_index,
        } => {
            assert_eq!(rows.len(), 34);
            assert!(used_index);
            assert_eq!(scanned, rows.len());
        }
        _ => panic!("预期 Select"),
    }
}

#[test]
fn drop_index_by_name() {
    let mut db = init_db();
    execute_sql(&mut db, "CREATE INDEX idx_age ON users (age)").unwrap();
    execute_sql(&mut db, "DROP INDEX idx_age ON users").unwrap();
    let res = execute_sql(&mut db, "SELECT * FROM users WHERE age = 25").unwrap();
    match res {
        ExecutionResult::Select {
            rows,
            scanned,
            used_index,
        } => {
            assert_eq!(rows.len(), 34);
            assert!(!used_index);
            assert_eq!(scanned, 1000);
        }
        _ => panic!("预期 Select"),
    }
}