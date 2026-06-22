mod common;
use common::init_users_table;
use wgc_db::{execute_sql, ExecutionResult, Database};
use tempfile::NamedTempFile;

/// 初始化一个包含 100 行 users 数据的数据库，保存 JSON 并清空 WAL（模拟干净关闭状态）
fn prepare_base(wal_path: &str, json_path: &str) {
    let mut db = Database::new(5);
    db.set_wal_path(wal_path);
    init_users_table(&mut db, 100);
    db.save(json_path).unwrap();
    wgc_db::database::wal::clear(wal_path).unwrap();
}

#[test]
fn test_auto_commit_recovery() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let json_file = NamedTempFile::new().unwrap();
    let json_path = json_file.path().to_str().unwrap().to_owned();

    prepare_base(&wal_path, &json_path);

    let mut db1 = Database::load(&json_path).unwrap();
    db1.set_wal_path(&wal_path);
    let count_before: usize =
        match execute_sql(&mut db1, "SELECT COUNT(*) FROM users").unwrap() {
            ExecutionResult::Select { rows, .. } => rows[0][0].parse().unwrap(),
            _ => panic!(),
        };
    assert_eq!(count_before, 100);

    execute_sql(&mut db1, "INSERT INTO users VALUES ('777', 'Auto', '77')").unwrap();

    let mut db2 = Database::load(&json_path).unwrap();
    db2.set_wal_path(&wal_path);
    db2.recover().unwrap();

    let res = execute_sql(&mut db2, "SELECT COUNT(*) FROM users").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            let count: usize = rows[0][0].parse().unwrap();
            assert_eq!(count, 101);
        }
        _ => panic!("Expected Select"),
    }
    let res = execute_sql(&mut db2, "SELECT * FROM users WHERE name = 'Auto'").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => assert_eq!(rows.len(), 1),
        _ => panic!("Expected Select"),
    }
}

#[test]
fn test_wal_recovery_after_commit() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let json_file = NamedTempFile::new().unwrap();
    let json_path = json_file.path().to_str().unwrap().to_owned();

    prepare_base(&wal_path, &json_path);

    let mut db1 = Database::load(&json_path).unwrap();
    db1.set_wal_path(&wal_path);

    execute_sql(&mut db1, "BEGIN").unwrap();
    execute_sql(&mut db1, "INSERT INTO users VALUES ('999', 'NewGuy', '99')").unwrap();
    execute_sql(&mut db1, "COMMIT").unwrap();

    let mut db2 = Database::load(&json_path).unwrap();
    db2.set_wal_path(&wal_path);
    db2.recover().unwrap();

    let res = execute_sql(&mut db2, "SELECT COUNT(*) FROM users").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            let count: usize = rows[0][0].parse().unwrap();
            assert_eq!(count, 101);
        }
        _ => panic!("Expected Select"),
    }
    let res = execute_sql(&mut db2, "SELECT * FROM users WHERE name = 'NewGuy'").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => assert_eq!(rows.len(), 1),
        _ => panic!("Expected Select"),
    }
}

#[test]
fn test_wal_recovery_after_rollback() {
    let wal_file = NamedTempFile::new().unwrap();
    let wal_path = wal_file.path().to_str().unwrap().to_owned();
    let json_file = NamedTempFile::new().unwrap();
    let json_path = json_file.path().to_str().unwrap().to_owned();

    prepare_base(&wal_path, &json_path);

    let mut db1 = Database::load(&json_path).unwrap();
    db1.set_wal_path(&wal_path);

    execute_sql(&mut db1, "BEGIN").unwrap();
    execute_sql(&mut db1, "INSERT INTO users VALUES ('111', 'Ghost', '11')").unwrap();
    execute_sql(&mut db1, "ROLLBACK").unwrap();

    let mut db2 = Database::load(&json_path).unwrap();
    db2.set_wal_path(&wal_path);
    db2.recover().unwrap();

    let res = execute_sql(&mut db2, "SELECT COUNT(*) FROM users").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            let count: usize = rows[0][0].parse().unwrap();
            assert_eq!(count, 100);
        }
        _ => panic!("Expected Select"),
    }
}