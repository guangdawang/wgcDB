mod database;
mod executor;

use database::Database;
use executor::execute_sql;

fn main() {
    let mut db = Database::new(5);

    // 1. 使用 SQL 创建表
    execute_sql(&mut db, "CREATE TABLE users (id INT, name TEXT, age INT)").unwrap();
    println!("表已创建");

    // 2. 批量插入数据（循环内构建 INSERT 语句）
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

    // 3. 多次查询触发自动索引
    let select_sql = "SELECT * FROM users WHERE name = 'Alice'";
    for i in 1..=12 {
        match execute_sql(&mut db, select_sql) {
            Ok(executor::ExecutionResult::Select { rows, scanned, used_index }) => {
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
}