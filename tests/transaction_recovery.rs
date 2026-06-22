use wgc_db::{execute_sql, ExecutionResult, Database};
use std::sync::Mutex;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn cleanup(files: &[&str]) {
    for f in files {
        let _ = std::fs::remove_file(f);
    }
}

/// 初始化一个包含 100 行 users 数据的数据库，保存 JSON 并清空 WAL（模拟干净关闭状态）
fn prepare_base(wal_path: &str, json_path: &str) {
    let mut db = Database::new(5);
    db.set_wal_path(wal_path);
    execute_sql(&mut db, "CREATE TABLE users (id INT, name TEXT, age INT)").unwrap();
    let names = ["Alice", "Bob", "Charlie", "Diana", "Eve"];
    for i in 0..100 {
        let sql = format!(
            "INSERT INTO users VALUES ('{}', '{}', '{}')",
            i,
            names[i % names.len()],
            20 + (i % 30)
        );
        execute_sql(&mut db, &sql).unwrap();
    }
    db.save(json_path).unwrap();
    // 清空 WAL，确保后续恢复只重放增量事务
    wgc_db::database::wal::clear(wal_path).unwrap();
}

#[test]
fn test_auto_commit_recovery() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let wal = "test_auto_commit.wal";
    let json = "test_auto_commit.json";
    cleanup(&[wal, json]);

    prepare_base(wal, json);

    let mut db1 = Database::load(json).unwrap();
    db1.set_wal_path(wal);
    let count_before: usize = match execute_sql(&mut db1, "SELECT COUNT(*) FROM users").unwrap() {
        ExecutionResult::Select { rows, .. } => rows[0][0].parse().unwrap(),
        _ => panic!(),
    };
    assert_eq!(count_before, 100);

    // 模拟运行时一条自动提交的 INSERT，然后崩溃
    execute_sql(&mut db1, "INSERT INTO users VALUES ('777', 'Auto', '77')").unwrap();

    // 模拟崩溃后重启：只加载 JSON，设置 WAL 并恢复
    let mut db2 = Database::load(json).unwrap();
    db2.set_wal_path(wal);
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

    cleanup(&[wal, json]);
}

#[test]
fn test_wal_recovery_after_commit() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let wal = "test_commit_rec.wal";
    let json = "test_commit_rec.json";
    cleanup(&[wal, json]);

    prepare_base(wal, json);

    let mut db1 = Database::load(json).unwrap();
    db1.set_wal_path(wal);

    execute_sql(&mut db1, "BEGIN").unwrap();
    execute_sql(&mut db1, "INSERT INTO users VALUES ('999', 'NewGuy', '99')").unwrap();
    execute_sql(&mut db1, "COMMIT").unwrap();

    // 崩溃后恢复
    let mut db2 = Database::load(json).unwrap();
    db2.set_wal_path(wal);
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

    cleanup(&[wal, json]);
}

#[test]
fn test_wal_recovery_after_rollback() {
    let _lock = TEST_MUTEX.lock().unwrap();
    let wal = "test_rollback_rec.wal";
    let json = "test_rollback_rec.json";
    cleanup(&[wal, json]);

    prepare_base(wal, json);

    let mut db1 = Database::load(json).unwrap();
    db1.set_wal_path(wal);

    execute_sql(&mut db1, "BEGIN").unwrap();
    execute_sql(&mut db1, "INSERT INTO users VALUES ('111', 'Ghost', '11')").unwrap();
    execute_sql(&mut db1, "ROLLBACK").unwrap();

    // 回滚后数据应保持不变
    let mut db2 = Database::load(json).unwrap();
    db2.set_wal_path(wal);
    db2.recover().unwrap();

    let res = execute_sql(&mut db2, "SELECT COUNT(*) FROM users").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            let count: usize = rows[0][0].parse().unwrap();
            assert_eq!(count, 100);
        }
        _ => panic!("Expected Select"),
    }

    cleanup(&[wal, json]);
}