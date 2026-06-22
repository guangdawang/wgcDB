mod common;
use common::init_db;
use wgc_db::{execute_sql, ExecutionResult};

#[test]
fn group_by_with_having() {
    let mut db = init_db();
    execute_sql(&mut db, "UPDATE users SET age = 99 WHERE name = 'Alice'").unwrap();
    execute_sql(&mut db, "DELETE FROM users WHERE age < 25").unwrap();

    let res =
        execute_sql(&mut db, "SELECT age, COUNT(*) FROM users GROUP BY age HAVING COUNT(*) > 2")
            .unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            assert_eq!(rows.len(), 21);
            let age_99 = rows.iter().find(|r| r[0] == "99").unwrap();
            assert_eq!(age_99[1], "200");
            let age_26 = rows.iter().find(|r| r[0] == "26").unwrap();
            assert_eq!(age_26[1], "34");
        }
        _ => panic!("预期 Select"),
    }
}

#[test]
fn self_join_limit() {
    let mut db = init_db();
    let res = execute_sql(
        &mut db,
        "SELECT u1.name, u2.name FROM users u1, users u2 WHERE u1.age = u2.age AND u1.id < u2.id LIMIT 5",
    )
    .unwrap();
    match res {
        ExecutionResult::Select { rows, .. } => {
            assert_eq!(rows.len(), 5);
            assert!(rows.iter().all(|r| r.len() == 2));
        }
        _ => panic!("预期 Select"),
    }
}