mod database;
mod executor;

use database::Database;
use executor::execute_sql;

const DB_FILE: &str = "wgcDB.json";

fn main() {
    // 尝试加载已有数据库，若不存在则新建
    let mut db = Database::load(DB_FILE).unwrap_or_else(|_| {
        println!("未找到数据文件，创建新数据库");
        Database::new(5)
    });

    // 如果表不存在则建表并插入数据（避免重复插入）
    if !db.tables.contains_key("users") {
        execute_sql(&mut db, "CREATE TABLE users (id INT, name TEXT, age INT)").unwrap();
        println!("表已创建");

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
        println!("1000 行数据已插入\n");
    } else {
        println!("从文件加载了已有数据库\n");
    }

    // 多次查询触发自动索引
    let select_sql = "SELECT * FROM users WHERE name = 'Alice'";
    for i in 1..=12 {
        match execute_sql(&mut db, select_sql) {
            Ok(executor::ExecutionResult::Select {
                rows,
                scanned,
                used_index,
            }) => {
                println!(
                    "查询 #{:<2}: 结果 {} 行 | 扫描 {} 行 | {}",
                    i,
                    rows.len(),
                    scanned,
                    if used_index { "📌 使用索引" } else { "🔍 全表扫描" }
                );
            }
            other => println!("其他结果: {:?}", other),
        }
    }

    // 退出前保存
    db.save(DB_FILE).expect("保存数据库失败");
    println!("\n数据库已保存到 {}", DB_FILE);
}