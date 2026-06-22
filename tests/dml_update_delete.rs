mod common;
use common::init_db;
use wgc_db::{execute_sql, ExecutionResult};

#[test]
fn update_and_verify() {
    let mut db = init_db();
    let res = execute_sql(&mut db, "UPDATE users SET age = 99 WHERE name = 'Alice'").unwrap();
    assert!(matches!(res, ExecutionResult::Update { count: 200 }));

    let res = execute_sql(&mut db, "SELECT * FROM users WHERE name = 'Alice'").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            assert_eq!(rows.len(), 200);
            assert!(rows.iter().all(|r| r[2] == "99"));
        }
        _ => panic!("预期 Select"),
    }
}

#[test]
fn delete_with_condition() {
    let mut db = init_db();
    execute_sql(&mut db, "UPDATE users SET age = 99 WHERE name = 'Alice'").unwrap();
    let res = execute_sql(&mut db, "DELETE FROM users WHERE age < 25").unwrap();
    assert!(matches!(res, ExecutionResult::Delete { count: 136 }));

    let res = execute_sql(&mut db, "SELECT COUNT(*) FROM users").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => assert_eq!(rows[0][0], "864"),
        _ => panic!("预期 Select"),
    }
}