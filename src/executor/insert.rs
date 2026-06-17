use super::ExecutionResult;
use crate::database::Database;
use crate::database::wal::WalRecord;
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