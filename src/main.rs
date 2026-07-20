// src/main.rs
mod server;   // 新增

use wgc_db::database;
use wgc_db::Database;

const DB_FILE: &str = "wgcDB";
const WAL_FILE: &str = "wgc_db.wal";
const SERVER_PORT: u16 = 9995;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--server" {
        server::run(SERVER_PORT);
        return;
    }

    // 原有命令行演示模式（保持不变）
    let mut db = Database::load(DB_FILE).unwrap_or_else(|_| {
        println!("未找到数据文件，创建新数据库");
        Database::new(5)
    });
    db.set_wal_path(WAL_FILE);

    db.recover().expect("WAL 恢复失败");

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

    println!("尝试显式事务...");
    wgc_db::execute_sql(&mut db, "BEGIN").unwrap();
    wgc_db::execute_sql(&mut db, "UPDATE users SET age = 99 WHERE name = 'Alice'").unwrap();
    wgc_db::execute_sql(&mut db, "DELETE FROM users WHERE age < 25").unwrap();
    wgc_db::execute_sql(&mut db, "COMMIT").unwrap();
    println!("事务提交完成");

    db.save(DB_FILE).expect("保存数据库失败");
    database::wal::clear(WAL_FILE).expect("清理 WAL 失败");
    println!("数据库已保存到 {}，WAL 已清除", DB_FILE);
}