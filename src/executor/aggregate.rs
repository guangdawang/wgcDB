use super::condition::check_condition;
use super::projection::{ProjectionInfo, ColumnExpr, apply_order_limit};
use super::join::TableRef;
use super::ExecutionResult;
use crate::database::Database;
use std::collections::HashMap;
use sqlparser::ast::*;
use super::aggregate_helpers::{compute_agg, evaluate_having};

pub fn execute_aggregate(
    db: &Database,
    table_ref: TableRef,
    conditions: &[super::condition::Condition],
    proj: &ProjectionInfo,
    group_by: &GroupByExpr,
    having: Option<&Expr>,
    order_by: &Option<OrderBy>,
    limit: &Option<LimitClause>,
) -> Result<ExecutionResult, String> {
    let table = db.tables.get(&table_ref.name).ok_or("Table not found")?;
    let all_columns = table.columns.clone();

    let filtered: Vec<&Vec<String>> = table.rows.values()
        .filter(|row| conditions.iter().all(|c| check_condition(row, &all_columns, c).unwrap_or(false)))
        .collect();
    let scanned = table.rows.len();

    let group_cols: Vec<String> = match group_by {
        GroupByExpr::Expressions(exprs, _) => {
            exprs.iter().map(|e| match e {
                Expr::Identifier(ident) => ident.value.clone(),
                _ => panic!("GROUP BY 仅支持列名"),
            }).collect()
        }
        GroupByExpr::All(_) => {
            all_columns.clone()
        }
    };

    let mut groups: HashMap<String, Vec<&Vec<String>>> = HashMap::new();
    for row in &filtered {
        let key = if group_cols.is_empty() {
            String::new()
        } else {
            group_cols.iter()
                .map(|c| {
                    let idx = all_columns.iter().position(|x| x == c).unwrap();
                    row[idx].clone()
                })
                .collect::<Vec<_>>()
                .join("\0")
        };
        groups.entry(key).or_default().push(row);
    }

    let mut output_rows = Vec::new();
    for (key, rows) in &groups {
        let mut out = Vec::new();
        for expr in &proj.columns {
            match expr {
                ColumnExpr::Column(col) => {
                    if let Some(idx) = group_cols.iter().position(|c| c == col) {
                        let val = key.split('\0').nth(idx).unwrap_or("");
                        out.push(val.to_string());
                    } else {
                        return Err(format!("非聚合列 '{}' 必须出现在 GROUP BY 中", col));
                    }
                }
                ColumnExpr::Aggregate(agg) => {
                    out.push(compute_agg(agg, rows, &all_columns)?);
                }
            }
        }
        if let Some(having_expr) = having {
            if !evaluate_having(having_expr, rows, &all_columns)? {
                continue;
            }
        }
        output_rows.push(out);
    }

    apply_order_limit(&mut output_rows, proj, &all_columns, order_by, limit)?;
    Ok(ExecutionResult::Select { rows: output_rows, scanned, used_index: false })
}