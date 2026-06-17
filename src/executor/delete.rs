use super::condition::extract_conditions;
use super::ExecutionResult;
use crate::database::Database;
use crate::database::wal::WalRecord;
use sqlparser::ast::*;

pub fn execute_delete(db: &mut Database, statement: &Statement) -> Result<(ExecutionResult, Option<WalRecord>), String> {
    if let Statement::Delete(delete) = statement {
        let table_name = if !delete.tables.is_empty() {
            delete.tables[0].to_string()
        } else {
            match &delete.from {
                FromTable::WithFromKeyword(from_tables) | FromTable::WithoutKeyword(from_tables) => {
                    if from_tables.is_empty() {
                        return Err("DELETE 缺少表名".into());
                    }
                    match &from_tables[0].relation {
                        TableFactor::Table { name, .. } => name.to_string(),
                        _ => return Err("仅支持简单表名".into()),
                    }
                }
            }
        };

        let conditions = if let Some(where_expr) = &delete.selection {
            extract_conditions(where_expr)?
        } else {
            vec![]
        };

        let deleted = db.tables.get_mut(&table_name)
            .ok_or("Table not found")?
            .delete_rows(&conditions)?;

        let wal = Some(WalRecord::Delete { table: table_name, conditions });
        Ok((ExecutionResult::Delete { count: deleted }, wal))
    } else {
        Err("Not a DELETE statement".into())
    }
}