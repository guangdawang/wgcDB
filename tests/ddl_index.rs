use wgc_db::{execute_sql, ExecutionResult, Database};

fn init_db() -> Database {
    let mut db = Database::new(5);
    execute_sql(&mut db, "CREATE TABLE users (id INT, name TEXT, age INT)").unwrap();
    let names = ["Alice", "Bob", "Charlie", "Diana", "Eve"];
    for i in 0..1000 {
        let sql = format!("INSERT INTO users VALUES ('{}', '{}', '{}')", i, names[i % names.len()], 20 + (i % 30));
        execute_sql(&mut db, &sql).unwrap();
    }
    db
}

#[test]
fn create_index_and_query() {
    let mut db = init_db();
    execute_sql(&mut db, "CREATE INDEX idx_age ON users (age)").unwrap();
    let (res, _) = execute_sql(&mut db, "SELECT * FROM users WHERE age = 25").unwrap();
    match res {
        ExecutionResult::Select { rows, scanned, used_index } => {
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

    // 查询应该不再使用索引，且结果应为 34 行（age=25 未被修改）
    let (res, _) = execute_sql(&mut db, "SELECT * FROM users WHERE age = 25").unwrap();
    match res {
        ExecutionResult::Select { rows, scanned, used_index } => {
            assert_eq!(rows.len(), 34);
            assert!(!used_index);          // 索引已删除
            assert_eq!(scanned, 1000);     // 全表扫描
        }
        _ => panic!("预期 Select"),
    }
}