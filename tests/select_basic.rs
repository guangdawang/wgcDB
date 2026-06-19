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
fn select_all_where_name_eq_alice() {
    let mut db = init_db();
    let res = execute_sql(&mut db, "SELECT * FROM users WHERE name = 'Alice'").unwrap();
    match res {
        ExecutionResult::Select { rows, scanned, used_index } => {
            assert_eq!(rows.len(), 200);
            assert_eq!(scanned, 1000);
            assert!(!used_index);
            assert_eq!(rows[0], vec!["0", "Alice", "20"]);
        }
        _ => panic!("预期 Select"),
    }
}

#[test]
fn select_with_multiple_conditions() {
    let mut db = init_db();
    let res = execute_sql(&mut db, "SELECT id, name FROM users WHERE age > 40 AND name = 'Bob'").unwrap();
    match res {
        ExecutionResult::Select { rows, scanned, .. } => {
            assert_eq!(rows.len(), 66);
            assert_eq!(scanned, 1000);
            assert_eq!(rows[0].len(), 2);
        }
        _ => panic!("预期 Select"),
    }
}

#[test]
fn select_range() {
    let mut db = init_db();
    let res = execute_sql(&mut db, "SELECT * FROM users WHERE age >= 30 AND age < 40").unwrap();
    match res {
        ExecutionResult::Select { rows, scanned, .. } => {
            assert_eq!(rows.len(), 330);
            assert_eq!(scanned, 1000);
        }
        _ => panic!("预期 Select"),
    }
}

#[test]
fn order_by_and_limit() {
    let mut db = init_db();
    let res = execute_sql(&mut db, "SELECT * FROM users ORDER BY age DESC LIMIT 3").unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[0][2], "49");
            let ages: Vec<i32> = rows.iter().map(|r| r[2].parse().unwrap()).collect();
            assert!(ages.windows(2).all(|w| w[0] >= w[1]));
        }
        _ => panic!("预期 Select"),
    }
}