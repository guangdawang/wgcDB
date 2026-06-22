use wgc_db::{execute_sql, ExecutionResult, Database};
use tempfile::NamedTempFile;

#[test]
fn test_commit_transaction() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let mut db = Database::new(5);
    db.set_wal_path(&wal_path);

    execute_sql(&mut db, "CREATE TABLE users (id INT, name TEXT, age INT)").unwrap();
    execute_sql(&mut db, "INSERT INTO users VALUES ('1', 'Alice', '20')").unwrap();

    assert!(matches!(
        execute_sql(&mut db, "BEGIN").unwrap(),
        ExecutionResult::BeginTransaction
    ));
    assert!(matches!(
        execute_sql(&mut db, "INSERT INTO users VALUES ('2', 'Bob', '30')").unwrap(),
        ExecutionResult::Pending
    ));
    assert!(matches!(
        execute_sql(&mut db, "UPDATE users SET age = 99 WHERE name = 'Alice'").unwrap(),
        ExecutionResult::Pending
    ));
    assert!(matches!(
        execute_sql(&mut db, "COMMIT").unwrap(),
        ExecutionResult::CommitTransaction
    ));

    let res = execute_sql(&mut db, "SELECT * FROM users WHERE name = 'Alice'").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0][2], "99");
        }
        _ => panic!("Expected Select"),
    }
    let res = execute_sql(&mut db, "SELECT * FROM users WHERE name = 'Bob'").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => assert_eq!(rows.len(), 1),
        _ => panic!("Expected Select"),
    }
}

#[test]
fn test_rollback_transaction() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let mut db = Database::new(5);
    db.set_wal_path(&wal_path);

    execute_sql(&mut db, "CREATE TABLE users (name TEXT, age INT)").unwrap();
    execute_sql(&mut db, "INSERT INTO users VALUES ('Eve', '40')").unwrap();

    execute_sql(&mut db, "BEGIN").unwrap();
    execute_sql(&mut db, "INSERT INTO users VALUES ('Frank', '50')").unwrap();
    execute_sql(&mut db, "UPDATE users SET age = 99 WHERE name = 'Eve'").unwrap();
    execute_sql(&mut db, "ROLLBACK").unwrap();

    let res = execute_sql(&mut db, "SELECT * FROM users").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0], vec!["Eve", "40"]);
        }
        _ => panic!("Expected Select"),
    }
}

#[test]
fn test_select_during_transaction() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let mut db = Database::new(5);
    db.set_wal_path(&wal_path);

    execute_sql(&mut db, "CREATE TABLE users (name TEXT)").unwrap();
    execute_sql(&mut db, "INSERT INTO users VALUES ('Alice')").unwrap();

    execute_sql(&mut db, "BEGIN").unwrap();
    execute_sql(&mut db, "INSERT INTO users VALUES ('Ghost')").unwrap();

    let res = execute_sql(&mut db, "SELECT * FROM users WHERE name = 'Ghost'").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => assert!(rows.is_empty()),
        _ => panic!("Expected Select"),
    }

    execute_sql(&mut db, "COMMIT").unwrap();

    let res = execute_sql(&mut db, "SELECT * FROM users WHERE name = 'Ghost'").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => assert_eq!(rows.len(), 1),
        _ => panic!("Expected Select"),
    }
}

#[test]
fn test_empty_transaction() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let mut db = Database::new(5);
    db.set_wal_path(&wal_path);

    execute_sql(&mut db, "BEGIN").unwrap();
    execute_sql(&mut db, "COMMIT").unwrap();

    execute_sql(&mut db, "BEGIN").unwrap();
    execute_sql(&mut db, "ROLLBACK").unwrap();
}

#[test]
fn test_nested_transaction_not_allowed() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let mut db = Database::new(5);
    db.set_wal_path(&wal_path);

    execute_sql(&mut db, "BEGIN").unwrap();
    let res = execute_sql(&mut db, "BEGIN");
    assert!(res.is_err());
    execute_sql(&mut db, "ROLLBACK").unwrap();
}

#[test]
fn test_ddl_in_transaction() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let mut db = Database::new(5);
    db.set_wal_path(&wal_path);

    execute_sql(&mut db, "BEGIN").unwrap();
    execute_sql(&mut db, "CREATE TABLE t1 (a INT)").unwrap();
    execute_sql(&mut db, "INSERT INTO t1 VALUES ('10')").unwrap();
    execute_sql(&mut db, "COMMIT").unwrap();

    let res = execute_sql(&mut db, "SELECT * FROM t1").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => assert_eq!(rows[0][0], "10"),
        _ => panic!("Expected Select"),
    }

    execute_sql(&mut db, "BEGIN").unwrap();
    execute_sql(&mut db, "CREATE TABLE t2 (b INT)").unwrap();
    execute_sql(&mut db, "ROLLBACK").unwrap();

    let res = execute_sql(&mut db, "SELECT * FROM t2");
    assert!(res.is_err());
}

#[test]
fn test_mixed_operations_in_transaction() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let mut db = Database::new(5);
    db.set_wal_path(&wal_path);

    execute_sql(&mut db, "CREATE TABLE items (id INT, val INT)").unwrap();
    execute_sql(&mut db, "INSERT INTO items VALUES ('1', '100')").unwrap();
    execute_sql(&mut db, "INSERT INTO items VALUES ('2', '200')").unwrap();

    execute_sql(&mut db, "BEGIN").unwrap();
    execute_sql(&mut db, "INSERT INTO items VALUES ('3', '300')").unwrap();
    execute_sql(&mut db, "UPDATE items SET val = 999 WHERE id = 1").unwrap();
    execute_sql(&mut db, "DELETE FROM items WHERE id = 2").unwrap();
    execute_sql(&mut db, "CREATE INDEX idx_val ON items (val)").unwrap();
    execute_sql(&mut db, "COMMIT").unwrap();

    let res = execute_sql(&mut db, "SELECT * FROM items ORDER BY id").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0], vec!["1", "999"]);
            assert_eq!(rows[1], vec!["3", "300"]);
        }
        _ => panic!("Expected Select"),
    }

    let res = execute_sql(&mut db, "SELECT * FROM items WHERE val = 300").unwrap();
    match res {
        ExecutionResult::Select { used_index, .. } => assert!(used_index),
        _ => panic!("Expected Select"),
    }
}