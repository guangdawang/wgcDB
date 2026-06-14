use super::ExecutionResult;
use crate::database::Database;
use sqlparser::ast::*;

pub fn execute_insert(db: &mut Database, statement: &Statement) -> Result<ExecutionResult, String> {
    if let Statement::Insert(insert) = statement {
        let table = db
            .tables
            .get_mut(&insert.table.to_string())
            .ok_or("Table not found")?;

        let values = match insert.source.as_ref() {
            Some(source) => match &*source.body {   // 解引用 Box<SetExpr>
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

        table.insert_row(row)?;
        Ok(ExecutionResult::Insert)
    } else {
        Err("Not an INSERT statement".into())
    }
}