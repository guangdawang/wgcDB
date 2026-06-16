use super::condition::{check_condition, compare_values, extract_conditions, Condition};
use super::ExecutionResult;
use crate::database::Database;
use sqlparser::ast::*;
use std::cmp::Ordering;

pub fn execute_select(db: &mut Database, query: &Query) -> Result<ExecutionResult, String> {
    let SetExpr::Select(select) = &*query.body else {
        return Err("Only simple SELECT".into());
    };
    let Select {
        projection,
        from,
        selection,
        ..
    } = select.as_ref();

    // ORDER BY 和 LIMIT 在 Query 层级
    let order_by = &query.order_by;           // Option<OrderBy>
    let limit_clause = &query.limit_clause;   // Option<LimitClause>

    // 解析投影列
    let mut projected_cols: Vec<String> = Vec::new();
    let mut is_wildcard = false;
    for item in projection {
        match item {
            SelectItem::Wildcard(_) => {
                is_wildcard = true;
                break;
            }
            SelectItem::UnnamedExpr(expr) => {
                if let Expr::Identifier(ident) = expr {
                    projected_cols.push(ident.value.clone());
                } else {
                    return Err("Only column names or * supported in SELECT".into());
                }
            }
            _ => return Err("Only column names or * supported in SELECT".into()),
        }
    }

    let table_name = match &from[0].relation {
        TableFactor::Table { name, .. } => name.to_string(),
        _ => return Err("Only simple table names".into()),
    };

    let table = db.tables.get(&table_name).ok_or("Table not found")?;
    let all_columns = table.columns.clone();

    if !is_wildcard {
        for col in &projected_cols {
            if !all_columns.contains(col) {
                return Err(format!("Column {} not found", col));
            }
        }
    }

    let conditions: Vec<Condition> = if let Some(where_expr) = selection {
        extract_conditions(where_expr)?
    } else {
        vec![]
    };

    // 尝试使用索引（仅单列等值条件）
    let mut used_index = false;
    let mut scanned = 0;
    let mut candidate_rows: Vec<usize> = Vec::new();

    if conditions.len() == 1 && conditions[0].op == BinaryOperator::Eq {
        let cond = &conditions[0];
        if let Some(index) = table.indexes.get(&cond.col) {
            candidate_rows = index.get(&cond.val).cloned().unwrap_or_default();
            scanned = candidate_rows.len();
            used_index = true;
        }
    }

    if !used_index {
        candidate_rows = (0..table.rows.len()).collect();
        scanned = table.rows.len();
    }

    // 应用所有条件
    let mut result_rows: Vec<Vec<String>> = Vec::new();
    for &row_id in &candidate_rows {
        let row = &table.rows[row_id];
        let mut ok = true;
        for cond in &conditions {
            if !check_condition(row, &all_columns, cond)? {
                ok = false;
                break;
            }
        }
        if ok {
            if is_wildcard {
                result_rows.push(row.clone());
            } else {
                let projected: Vec<String> = projected_cols
                    .iter()
                    .map(|c| {
                        let idx = all_columns.iter().position(|x| x == c).unwrap();
                        row[idx].clone()
                    })
                    .collect();
                result_rows.push(projected);
            }
        }
    }

    // ORDER BY（支持单列 ASC/DESC）
    if let Some(order_by) = order_by {
        let exprs: &[OrderByExpr] = match &order_by.kind {
            OrderByKind::Expressions(exprs) => exprs.as_slice(),
            OrderByKind::All(_) => &[],
        };
        if exprs.len() == 1 {
            let ob = &exprs[0];
            let col_name = match &ob.expr {
                Expr::Identifier(ident) => ident.value.clone(),
                _ => return Err("ORDER BY only supports column name".into()),
            };
            let ascending = ob.options.asc.unwrap_or(true);
            result_rows.sort_by(|a, b| {
                if is_wildcard {
                    let col_idx = all_columns.iter().position(|c| c == &col_name).unwrap_or(0);
                    let va = &a[col_idx];
                    let vb = &b[col_idx];
                    if ascending {
                        compare_values(va, vb)
                    } else {
                        compare_values(vb, va)
                    }
                } else {
                    let pos = projected_cols.iter().position(|c| c == &col_name);
                    match pos {
                        Some(p) => {
                            let va = &a[p];
                            let vb = &b[p];
                            if ascending {
                                compare_values(va, vb)
                            } else {
                                compare_values(vb, va)
                            }
                        }
                        None => Ordering::Equal,
                    }
                }
            });
        } else if !exprs.is_empty() {
            return Err("Only single column ORDER BY supported".into());
        }
    }

    // LIMIT
    if let Some(limit_clause) = limit_clause {
        match limit_clause {
            LimitClause::LimitOffset { limit, .. } => {
                if let Some(limit_expr) = limit {
                    if let Expr::Value(ValueWithSpan {
                        value: Value::Number(n, _),
                        ..
                    }) = limit_expr
                    {
                        let limit_num: usize =
                            n.parse().map_err(|_| "Invalid LIMIT value".to_string())?;
                        result_rows.truncate(limit_num);
                    } else {
                        return Err("LIMIT must be a number".into());
                    }
                }
            }
            LimitClause::OffsetCommaLimit { limit, .. } => {
                if let Expr::Value(ValueWithSpan {
                    value: Value::Number(n, _),
                    ..
                }) = limit
                {
                    let limit_num: usize =
                        n.parse().map_err(|_| "Invalid LIMIT value".to_string())?;
                    result_rows.truncate(limit_num);
                } else {
                    return Err("LIMIT must be a number".into());
                }
            }
        }
    }

    // 自动索引统计
    for cond in &conditions {
        if cond.op == BinaryOperator::Eq {
            let hint = db.stats.record_and_check(&table_name, &cond.col, used_index);
            if let Some((tbl, col)) = hint {
                if let Some(tbl_ref) = db.tables.get_mut(&tbl) {
                    if !tbl_ref.indexes.contains_key(&col) {
                        tbl_ref.build_index(&col)?;
                        println!("⚡ 自动创建索引: {}.{}", tbl, col);
                    }
                }
            }
        }
    }

    Ok(ExecutionResult::Select {
        rows: result_rows,
        scanned,
        used_index,
    })
}