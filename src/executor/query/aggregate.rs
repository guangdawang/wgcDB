use crate::database::Database;
use crate::core::condition::check_condition;
use crate::executor::projection::{ProjectionInfo, ColumnExpr, AggregateFunction, apply_order_limit};
use crate::executor::ExecutionResult;
use super::join::TableRef;
use std::collections::HashMap;
use sqlparser::ast::*;

pub fn execute_aggregate(
    db: &Database,
    table_ref: TableRef,
    conditions: &[crate::core::condition::Condition],
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

fn compute_agg(func: &AggregateFunction, rows: &[&Vec<String>], columns: &[String]) -> Result<String, String> {
    match func {
        AggregateFunction::Count(arg) => {
            if arg == "*" {
                return Ok(rows.len().to_string());
            }
            let idx = columns.iter().position(|c| c == arg).ok_or("列不存在")?;
            let count = rows.iter().filter(|r| r[idx] != "NULL").count();
            Ok(count.to_string())
        }
        AggregateFunction::Sum(arg) => {
            let idx = columns.iter().position(|c| c == arg).ok_or("列不存在")?;
            let sum: f64 = rows.iter()
                .filter_map(|r| r[idx].parse::<f64>().ok())
                .sum();
            Ok(sum.to_string())
        }
        AggregateFunction::Avg(arg) => {
            let idx = columns.iter().position(|c| c == arg).ok_or("列不存在")?;
            let (count, sum) = rows.iter()
                .filter_map(|r| r[idx].parse::<f64>().ok())
                .fold((0u64, 0.0), |(c, s), v| (c+1, s+v));
            if count == 0 { return Ok("0".into()); }
            Ok((sum / count as f64).to_string())
        }
    }
}

fn evaluate_having(expr: &Expr, rows: &[&Vec<String>], columns: &[String]) -> Result<bool, String> {
    if let Expr::BinaryOp { left, op, right } = expr {
        let left_val = eval_having_expr(left, rows, columns)?;
        let right_val = eval_having_expr(right, rows, columns)?;
        let cmp = if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
            l.partial_cmp(&r)
        } else {
            Some(left_val.cmp(&right_val))
        };
        match cmp {
            Some(std::cmp::Ordering::Equal) => Ok(op == &BinaryOperator::Eq),
            Some(std::cmp::Ordering::Less) => Ok(op == &BinaryOperator::Lt),
            Some(std::cmp::Ordering::Greater) => Ok(op == &BinaryOperator::Gt),
            _ => Ok(false),
        }
    } else {
        Err("HAVING 仅支持简单比较".into())
    }
}

fn eval_having_expr(expr: &Expr, rows: &[&Vec<String>], columns: &[String]) -> Result<String, String> {
    match expr {
        Expr::Value(ValueWithSpan { value, .. }) => match value {
            Value::Number(n, _) => Ok(n.clone()),
            Value::SingleQuotedString(s) => Ok(s.clone()),
            _ => Err("不支持的常量".into()),
        },
        Expr::Function(func) => {
            let name = func.name.to_string().to_uppercase();
            let args = match &func.args {
                FunctionArguments::List(list) => &list.args,
                _ => return Err("不支持的函数参数".into()),
            };
            let arg = if args.is_empty() {
                "*".into()
            } else {
                match &args[0] {
                    FunctionArg::Unnamed(arg_expr) => match arg_expr {
                        FunctionArgExpr::Wildcard => "*".into(),
                        FunctionArgExpr::Expr(Expr::Identifier(id)) => id.value.clone(),
                        _ => return Err("参数错误".into()),
                    },
                    _ => return Err("参数错误".into()),
                }
            };
            match name.as_str() {
                "COUNT" => compute_agg(&AggregateFunction::Count(arg), rows, columns),
                "SUM" => compute_agg(&AggregateFunction::Sum(arg), rows, columns),
                "AVG" => compute_agg(&AggregateFunction::Avg(arg), rows, columns),
                _ => Err("不支持的聚合函数".into()),
            }
        }
        _ => Err("HAVING 表达式过于复杂".into()),
    }
}