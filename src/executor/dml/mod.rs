use crate::database::Database;
use crate::database::wal::WalRecord;
use crate::core::condition::extract_conditions;
use crate::executor::ExecutionResult;
use sqlparser::ast::*;

pub fn execute_insert(db: &mut Database, statement: &Statement) -> Result<(ExecutionResult, Option<WalRecord>), String> {
    if let Statement::Insert(insert) = statement {
        let table_name = insert.table.to_string();
        let table = db.tables.get_mut(&table_name).ok_or("Table not found")?;

        let values = match insert.source.as_ref() {
            Some(source) => match &*source.body {
                SetExpr::Values(vals) => &vals.rows[0],
                _ => return Err("Only VALUES() supported".into()),
            },
            None => return Err("Missing source".into()),
        };

        let row: Vec<String> = values
            .iter()
            .map(|v| match v {
                Expr::Value(ValueWithSpan { value, .. }) => match value {
                    Value::SingleQuotedString(s) => s.clone(),
                    Value::Number(n, _) => n.clone(),
                    _ => "NULL".to_string(),
                },
                _ => "NULL".to_string(),
            })
            .collect();

        table.insert_row(row.clone())?;
        let wal = Some(WalRecord::Insert {
            table: table_name,
            values: row,
        });
        Ok((ExecutionResult::Insert { count: 1 }, wal))
    } else {
        Err("Not an INSERT statement".into())
    }
}

pub fn execute_update(db: &mut Database, statement: &Statement) -> Result<(ExecutionResult, Option<WalRecord>), String> {
    if let Statement::Update(update) = statement {
        let table_name = update.table.to_string();
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