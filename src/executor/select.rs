use super::ExecutionResult;
use crate::database::Database;
use sqlparser::ast::*;

pub fn execute_select(db: &mut Database, query: &Query) -> Result<ExecutionResult, String> {
    let Select {
        projection,
        from,
        selection,
        ..
    } = match &*query.body {
        SetExpr::Select(select) => select.as_ref(),
        _ => return Err("Only simple SELECT".into()),
    };

    if !matches!(projection[0], SelectItem::Wildcard(_)) {
        return Err("Only SELECT * supported".into());
    }

    let table_name = match &from[0].relation {
        TableFactor::Table { name, .. } => name.to_string(),
        _ => return Err("Only simple table names".into()),
    };

    let table = db.tables.get(&table_name).ok_or("Table not found")?;

    // ---- 解析 WHERE 子句 ----
    let selection = match selection {
        Some(expr) => expr,
        None => return Err("WHERE clause required".into()),
    };

    let (filter_col, filter_val) = match selection {
        Expr::BinaryOp { left, op, right } => {
            if *op != BinaryOperator::Eq {
                return Err("Only = supported".into());
            }
            let col = if let Expr::Identifier(ident) = &**left {
                ident.value.clone()
            } else {
                return Err("Left side must be column".into());
            };
            let val = match &**right {
                Expr::Value(ValueWithSpan { value, .. }) => match value {
                    Value::SingleQuotedString(s) => s.clone(),
                    Value::Number(n, _) => n.clone(),
                    _ => return Err("Unsupported literal".into()),
                },
                _ => return Err("Right side must be literal".into()),
            };
            (col, val)
        }
        _ => return Err("WHERE must be binary op".into()),
    };
    // -------------------------

    // 判断是否使用索引
    let (rows, scanned, used_index) = if let Some(index) = table.indexes.get(&filter_col) {
        let ids = index.get(&filter_val).cloned().unwrap_or_default();
        let scanned = ids.len();
        let rows = ids.iter().map(|&id| table.rows[id].clone()).collect();
        (rows, scanned, true)
    } else {
        let col_idx = table
            .columns
            .iter()
            .position(|c| c == &filter_col)
            .ok_or("Column not found")?;
        let mut rows = Vec::new();
        for row in &table.rows {
            if row[col_idx] == filter_val {
                rows.push(row.clone());
            }
        }
        (rows, table.rows.len(), false)
    };

    // 更新统计并可能自动建索引
    let index_hint = db.stats.record_and_check(&table_name, &filter_col, used_index);
    if let Some((tbl, col)) = index_hint {
        if let Some(table) = db.tables.get_mut(&tbl) {
            if !table.indexes.contains_key(&col) {
                table.build_index(&col)?;
                println!("⚡ 自动创建索引: {}.{}", tbl, col);
            }
        }
    }

    Ok(ExecutionResult::Select {
        rows,
        scanned,
        used_index,
    })
}