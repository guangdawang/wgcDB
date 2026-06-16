use super::condition::extract_conditions;
use super::ExecutionResult;
use crate::database::Database;
use crate::wal::WalRecord;
use sqlparser::ast::*;

pub fn execute_update(db: &mut Database, statement: &Statement) -> Result<(ExecutionResult, Option<WalRecord>), String> {
    if let Statement::Update(update) = statement {
        let table_name = update.table.to_string();
        // AssignmentTarget 可能为 ColumnName 或 Tuple
        let set_col = match &update.assignments[0].target {
            AssignmentTarget::ColumnName(ident) => ident.to_string(),
            _ => return Err("SET 目标仅支持简单列名".into()),
        };
        let set_val = match &update.assignments[0].value {
            Expr::Value(ValueWithSpan { value, .. }) => match value {
                Value::SingleQuotedString(s) => s.clone(),
                Value::Number(n, _) => n.clone(),
                _ => return Err("Unsupported value in SET".into()),
            },
            _ => return Err("SET value must be literal".into()),
        };
        let conditions = if let Some(where_expr) = &update.selection {
            extract_conditions(where_expr)?
        } else {
            vec![]
        };

        let updated = db.tables.get_mut(&table_name)
            .ok_or("Table not found")?
            .update_rows(&conditions, &set_col, &set_val)?;

        let wal = Some(WalRecord::Update { table: table_name, conditions, set_col, set_val });
        Ok((ExecutionResult::Update { count: updated }, wal))
    } else {
        Err("Not an UPDATE statement".into())
    }
}