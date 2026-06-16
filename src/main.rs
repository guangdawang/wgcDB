mod database;
mod executor;

use database::Database;
use executor::execute_sql;

const DB_FILE: &str = "wgcDB.json";

fn main() {
    let mut db = Database::load(DB_FILE).unwrap_or_else(|_| {
        println!("未找到数据文件，创建新数据库");
        Database::new(5)
    });

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

    // 测试各种新功能
    let tests = vec![
        "SELECT * FROM users WHERE name = 'Alice'",
        "SELECT id, name FROM users WHERE age > 40 AND name = 'Bob'",
        "SELECT * FROM users WHERE age >= 30 AND age < 40",
        "SELECT * FROM users ORDER BY age DESC LIMIT 3",
        "CREATE INDEX idx_age ON users (age)",
        "SELECT * FROM users WHERE age = 25",
    ];

    for sql in tests {
        println!("SQL: {}", sql);
        match execute_sql(&mut db, sql) {
            Ok(executor::ExecutionResult::Select {
                rows,
                scanned,
                used_index,
            }) => {
                println!(
                    "  结果: {} 行 | 扫描: {} | 索引: {}",
                    rows.len(),
                    scanned,
                    if used_index { "✅" } else { "❌" }
                );
                // 只显示前5行
                for row in rows.iter().take(5) {
                    println!("    {:?}", row);
                }
                if rows.len() > 5 {
                    println!("    ... 共 {} 行", rows.len());
                }
            }
            Ok(other) => println!("  {:?}", other),
            Err(e) => println!("  错误: {}", e),
        }
        println!();
    }

    db.save(DB_FILE).expect("保存数据库失败");
    println!("数据库已保存到 {}", DB_FILE);
}