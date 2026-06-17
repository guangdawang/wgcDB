mod ddl;
mod dml;
mod query;

pub mod condition;
pub mod projection;
pub mod projection_parse;

use crate::database::Database;
use crate::database::wal::WalRecord;
use sqlparser::ast::{ObjectType, Statement};

#[derive(Debug)]
pub enum ExecutionResult {
    Select { rows: Vec<Vec<String>>, scanned: usize, used_index: bool },
    Insert { count: usize },
    Update { count: usize },
    Delete { count: usize },
    CreateTable,
    CreateIndex,
    DropIndex,
}

pub fn execute_sql(db: &mut Database, sql: &str) -> Result<(ExecutionResult, Option<WalRecord>), String> {
    let dialect = sqlparser::dialect::GenericDialect {};
    let ast = sqlparser::parser::Parser::parse_sql(&dialect, sql)
        .map_err(|e| format!("Parse error: {e}"))?;

    if ast.len() != 1 {
        return Err("Only one statement per call is supported".into());
    }

    match &ast[0] {
        Statement::Query(query) => {
            let res = query::execute_select(db, query)?;
            Ok((res, None))
        }
        Statement::Insert(_) => dml::execute_insert(db, &ast[0]),
        Statement::Update(_) => dml::execute_update(db, &ast[0]),
        Statement::Delete(_) => dml::execute_delete(db, &ast[0]),
        Statement::CreateTable(_) | Statement::CreateIndex(_) => ddl::execute_create(db, &ast[0]),
        Statement::Drop { object_type, names, table, .. } => {
            if *object_type == ObjectType::Index {
                ddl::execute_drop_index(db, names, table)
            } else {
                Err("Only DROP INDEX is supported".into())
            }
        }
        _ => Err("Unsupported statement".into()),
    }
}