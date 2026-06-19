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

    // 应用 WAL
    match database::wal::read_records(WAL_FILE) {
        Ok(records) => {
            for rec in &records {
                db.apply_wal_record(rec)
                    .unwrap_or_else(|e| eprintln!("WAL 重放警告: {}", e));
            }
            if !records.is_empty() {
                println!("已从 WAL 重放 {} 条操作", records.len());
            }
        }
        Err(e) => eprintln!("WAL 读取失败: {}", e),
    }

    // 如果不存在则初始化示例数据（仅用于演示，测试不依赖此数据）
    if !db.tables.contains_key("users") {
        let _ = wgc_db::execute_sql(&mut db, "CREATE TABLE users (id INT, name TEXT, age INT)");
        let names = ["Alice", "Bob", "Charlie", "Diana", "Eve"];
        for i in 0..1000 {
            let sql = format!(
                "INSERT INTO users VALUES ('{}', '{}', '{}')",
                i,
                names[i % names.len()],
                20 + (i % 30)
            );
            let _ = wgc_db::execute_sql(&mut db, &sql);
        }
        println!("示例数据已初始化");
    }

    // 这里可以扩展为 REPL 或直接保存后退出
    db.save(DB_FILE).expect("保存数据库失败");
    database::wal::clear(WAL_FILE).expect("清理 WAL 失败");
    println!("数据库已保存到 {}，WAL 已清除", DB_FILE);
}