mod create;
mod insert;
mod select;

use crate::database::Database;
use sqlparser::ast::Statement;

#[derive(Debug)]
pub enum ExecutionResult {
    Select {
        rows: Vec<Vec<String>>,
        scanned: usize,
        used_index: bool,
    },
    Insert,
    CreateTable,
}

/// 解析并执行单条 SQL 语句
pub fn execute_sql(db: &mut Database, sql: &str) -> Result<ExecutionResult, String> {
    let dialect = sqlparser::dialect::GenericDialect {};
    let ast = sqlparser::parser::Parser::parse_sql(&dialect, sql)
        .map_err(|e| format!("Parse error: {e}"))?;

    if ast.len() != 1 {
        return Err("Only one statement per call is supported".into());
    }

    match &ast[0] {
        Statement::Query(query) => select::execute_select(db, query),
        Statement::Insert(_) => insert::execute_insert(db, &ast[0]),
        Statement::CreateTable(_) => create::execute_create(db, &ast[0]),
        _ => Err("Unsupported statement".into()),
    }
}