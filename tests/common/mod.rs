use wgc_db::{execute_sql, Database};

/// 创建包含 1000 条用户数据的数据库
#[allow(dead_code)]
pub fn init_db() -> Database {
    let mut db = Database::new(5);
    execute_sql(&mut db, "CREATE TABLE users (id INT, name TEXT, age INT)").unwrap();
    let names = ["Alice", "Bob", "Charlie", "Diana", "Eve"];
    for i in 0..1000 {
        let sql = format!(
            "INSERT INTO users VALUES ('{}', '{}', '{}')",
            i,
            names[i % names.len()],
            20 + (i % 30)
        );
        execute_sql(&mut db, &sql).unwrap();
    }
    db
}

/// 在已有数据库中插入指定行数的用户数据（用于事务恢复测试）
#[allow(dead_code)]
pub fn init_users_table(db: &mut Database, rows: usize) {
    execute_sql(db, "CREATE TABLE users (id INT, name TEXT, age INT)").unwrap();
    let names = ["Alice", "Bob", "Charlie", "Diana", "Eve"];
    for i in 0..rows {
        let sql = format!(
            "INSERT INTO users VALUES ('{}', '{}', '{}')",
            i,
            names[i % names.len()],
            20 + (i % 30)
        );
        execute_sql(db, &sql).unwrap();
    }
}