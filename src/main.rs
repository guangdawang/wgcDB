mod database;
mod executor;
mod wal;

use database::Database;
use executor::{execute_sql, ExecutionResult};

const DB_FILE: &str = "wgcDB.json";
const WAL_FILE: &str = "wgcDB.wal";

fn main() {
    let mut db = Database::load(DB_FILE).unwrap_or_else(|_| {
        println!("未找到数据文件，创建新数据库");
        Database::new(5)
    });

    // 应用 WAL
    match wal::read_records(WAL_FILE) {
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

    // 初始化示例数据
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

    let tests = vec![
        "SELECT * FROM users WHERE name = 'Alice'",
        "SELECT id, name FROM users WHERE age > 40 AND name = 'Bob'",
        "SELECT * FROM users WHERE age >= 30 AND age < 40",
        "SELECT * FROM users ORDER BY age DESC LIMIT 3",
        "CREATE INDEX idx_age ON users (age)",
        "SELECT * FROM users WHERE age = 25",
        "UPDATE users SET age = 99 WHERE name = 'Alice'",
        "SELECT * FROM users WHERE name = 'Alice'",
        "DELETE FROM users WHERE age < 25",
        "SELECT COUNT(*) FROM users",
        "SELECT age, COUNT(*) FROM users GROUP BY age HAVING COUNT(*) > 2",
        "SELECT u1.name, u2.name FROM users u1, users u2 WHERE u1.age = u2.age AND u1.id < u2.id LIMIT 5",
        "DROP INDEX idx_age ON users",
    ];

    for sql in tests {
        println!("SQL: {}", sql);
        match execute_sql(&mut db, sql) {
            Ok((result, wal_opt)) => {
                // 显式使用所有字段，消除 dead_code 警告
                match &result {
                    ExecutionResult::Select { rows, scanned, used_index } => {
                        println!("  Select {{ rows: [...], scanned: {}, used_index: {} }}", scanned, used_index);
                        for row in rows.iter().take(5) {
                            println!("    {:?}", row);
                        }
                        if rows.len() > 5 {
                            println!("    ... 共 {} 行", rows.len());
                        }
                    }
                    ExecutionResult::Insert { count } => {
                        println!("  Insert {{ count: {} }}", count);
                    }
                    ExecutionResult::Update { count } => {
                        println!("  Update {{ count: {} }}", count);
                    }
                    ExecutionResult::Delete { count } => {
                        println!("  Delete {{ count: {} }}", count);
                    }
                    ExecutionResult::CreateTable => {
                        println!("  CreateTable");
                    }
                    ExecutionResult::CreateIndex => {
                        println!("  CreateIndex");
                    }
                    ExecutionResult::DropIndex => {
                        println!("  DropIndex");
                    }
                }
                // 写入 WAL
                if let Some(rec) = wal_opt {
                    wal::append_record(WAL_FILE, &rec)
                        .unwrap_or_else(|e| eprintln!("WAL 写入失败: {}", e));
                }
            }
            Err(e) => println!("  错误: {}", e),
        }
        println!();
    }

    // 保存快照，清空 WAL
    db.save(DB_FILE).expect("保存数据库失败");
    wal::clear(WAL_FILE).expect("清理 WAL 失败");
    println!("数据库已保存到 {}，WAL 已清除", DB_FILE);
}