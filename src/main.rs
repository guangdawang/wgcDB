// src/main.rs
use wgc_db::database;
use wgc_db::Database;

const DB_FILE: &str = "wgc_db.json";
const WAL_FILE: &str = "wgc_db.wal";

fn main() {
    let mut db = Database::load(DB_FILE).unwrap_or_else(|_| {
        println!("未找到数据文件，创建新数据库");
        Database::new(5)
    });
    db.set_wal_path(WAL_FILE);

    // 从 WAL 恢复（重放完整事务）
    db.recover().expect("WAL 恢复失败");

    // 如果不存在则初始化示例数据（仅用于演示，测试不依赖此数据）
    if !db.tables.contains_key("users") {
        let _ = wgc_db::execute_sql(&mut db, "CREATE TABLE users (id INT, name TEXT, age INT)").unwrap();
        let names = ["Alice", "Bob", "Charlie", "Diana", "Eve"];
        for i in 0..1000 {
            let sql = format!(
                "INSERT INTO users VALUES ('{}', '{}', '{}')",
                i,
                names[i % names.len()],
                20 + (i % 30)
            );
            let _ = wgc_db::execute_sql(&mut db, &sql).unwrap();
        }
        println!("示例数据已初始化");
    }

    // 简单演示事务
    println!("尝试显式事务...");
    wgc_db::execute_sql(&mut db, "BEGIN").unwrap();
    wgc_db::execute_sql(&mut db, "UPDATE users SET age = 99 WHERE name = 'Alice'").unwrap();
    wgc_db::execute_sql(&mut db, "DELETE FROM users WHERE age < 25").unwrap();
    wgc_db::execute_sql(&mut db, "COMMIT").unwrap();
    println!("事务提交完成");

    // 最终保存并清除 WAL
    db.save(DB_FILE).expect("保存数据库失败");
    database::wal::clear(WAL_FILE).expect("清理 WAL 失败");
    println!("数据库已保存到 {}，WAL 已清除", DB_FILE);
}