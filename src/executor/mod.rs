mod ddl;
mod dml;
mod query;

pub mod projection;

use crate::database::Database;
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
    /// 显式事务中暂存的操作，无实际影响
    Pending,
    BeginTransaction,
    CommitTransaction,
    RollbackTransaction,
}

pub fn execute_sql(db: &mut Database, sql: &str) -> Result<ExecutionResult, String> {
    let dialect = sqlparser::dialect::GenericDialect {};
    let ast = sqlparser::parser::Parser::parse_sql(&dialect, sql)
        .map_err(|e| format!("Parse error: {e}"))?;

    if ast.len() != 1 {
        return Err("Only one statement per call is supported".into());
    }

    let statement = &ast[0];

    // 事务控制
    match statement {
        Statement::StartTransaction { .. } => {
            db.begin_transaction()?;
            return Ok(ExecutionResult::BeginTransaction);
        }
        Statement::Commit { .. } => {
            db.commit_transaction()?;
            return Ok(ExecutionResult::CommitTransaction);
        }
        Statement::Rollback { chain: _, .. } => {
            // sqlparser 中 Rollback 的语法包括 chain 等，这里忽略
            db.rollback_transaction()?;
            return Ok(ExecutionResult::RollbackTransaction);
        }
        _ => {}
    }

    // 查询语句（即使在事务中也直接基于当前已提交数据执行）
    if let Statement::Query(query) = statement {
        return query::execute_select(db, query);
    }

    // 构造写操作记录
    let record = match statement {
        Statement::Insert(_) => dml::make_insert_record(statement)?,
        Statement::Update(_) => dml::make_update_record(statement)?,
        Statement::Delete(_) => dml::make_delete_record(statement)?,
        Statement::CreateTable(_) | Statement::CreateIndex(_) => {
            ddl::make_create_record(statement)?
        }
        Statement::Drop { object_type, names, table, .. }
            if *object_type == ObjectType::Index =>
        {
            ddl::make_drop_index_record(names, table)?
        }
        _ => return Err("Unsupported statement".into()),
    };

    // 统一交给 Database::handle_write 处理（事务感知）
    db.handle_write(record)
}