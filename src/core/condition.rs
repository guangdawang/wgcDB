use serde::{Deserialize, Serialize};
use sqlparser::ast::*;
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub col: String,              // 左侧列名（可能含表别名，如 "u1.age"）
    pub op: String,
    pub val: String,              // 右侧值（字面量 或 列名，由 rhs_is_column 决定）
    #[serde(default)]
    pub rhs_is_column: bool,      // true 表示 val 是列名，false 是字面量
}

fn op_to_str(op: &BinaryOperator) -> &'static str {
    match op {
        BinaryOperator::Eq => "=",
        BinaryOperator::NotEq => "!=",
        BinaryOperator::Lt => "<",
        BinaryOperator::LtEq => "<=",
        BinaryOperator::Gt => ">",
        BinaryOperator::GtEq => ">=",
        _ => "unknown",
    }
}

fn compare_with_op(op: &str, cmp: Ordering) -> bool {
    match op {
        "="  => cmp == Ordering::Equal,
        "!=" => cmp != Ordering::Equal,
        "<"  => cmp == Ordering::Less,
        "<=" => cmp == Ordering::Less || cmp == Ordering::Equal,
        ">"  => cmp == Ordering::Greater,
        ">=" => cmp == Ordering::Greater || cmp == Ordering::Equal,
        _ => false,
    }
}

pub fn extract_conditions(expr: &Expr) -> Result<Vec<Condition>, String> {
    match expr {
        Expr::BinaryOp { left, op, right } if *op == BinaryOperator::And => {
            let mut left_conds = extract_conditions(left)?;
            let mut right_conds = extract_conditions(right)?;
            left_conds.append(&mut right_conds);
            Ok(left_conds)
        }
        Expr::BinaryOp { left, op, right } => {
            let col = extract_column_name(left)?;
            let op_str = op_to_str(op).to_string();

            // 判断右侧是列还是字面量
            if let Ok(col_name) = extract_column_name(right) {
                // 右侧是列
                Ok(vec![Condition {
                    col,
                    op: op_str,
                    val: col_name,
                    rhs_is_column: true,
                }])
            } else if let Ok(lit_val) = extract_literal_value(right) {
                // 右侧是字面量
                Ok(vec![Condition {
                    col,
                    op: op_str,
                    val: lit_val,
                    rhs_is_column: false,
                }])
            } else {
                Err("Right side must be a literal or column".into())
            }
        }
        _ => Err("Unsupported WHERE clause format".into()),
    }
}

fn extract_column_name(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Identifier(ident) => Ok(ident.value.clone()),
        Expr::CompoundIdentifier(parts) => {
            let name = parts.iter()
                .map(|p| p.value.clone())
                .collect::<Vec<_>>()
                .join(".");
            Ok(name)
        }
        _ => Err("Not a column name".into()),
    }
}

fn extract_literal_value(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Value(ValueWithSpan { value, .. }) => match value {
            Value::SingleQuotedString(s) => Ok(s.clone()),
            Value::Number(n, _) => Ok(n.clone()),
            Value::Boolean(b) => Ok(b.to_string()),
            _ => Err("Unsupported literal value".into()),
        },
        _ => Err("Not a literal".into()),
    }
}

/// 单表查询中的条件检查（不支持列对列，此时 rhs_is_column 必须为 false）
pub fn check_condition(row: &[String], columns: &[String], cond: &Condition) -> Result<bool, String> {
    if cond.rhs_is_column {
        return Err("Column-vs-column comparison not allowed in single-table scan".into());
    }
    let actual_col = extract_actual_column(&cond.col);
    let col_idx = columns.iter().position(|c| c == actual_col)
        .ok_or(format!("Column {} not found", actual_col))?;
    let row_val = &row[col_idx];
    let cmp = compare_vals(row_val, &cond.val);
    match cmp {
        Some(ord) => Ok(compare_with_op(&cond.op, ord)),
        None => Ok(false),
    }
}

/// 从可能带表别名的列名中提取实际列名（取最后一段）
pub fn extract_actual_column(full_name: &str) -> &str {
    full_name.rfind('.').map(|pos| &full_name[pos+1..]).unwrap_or(full_name)
}

/// 比较两个字符串值，尝试按数值比较，否则按字符串比较
pub fn compare_values(a: &str, b: &str) -> Ordering {
    if let (Ok(n1), Ok(n2)) = (a.parse::<f64>(), b.parse::<f64>()) {
        n1.partial_cmp(&n2).unwrap_or(Ordering::Equal)
    } else {
        a.cmp(b)
    }
}

fn compare_vals(a: &str, b: &str) -> Option<Ordering> {
    if let (Ok(n1), Ok(n2)) = (a.parse::<f64>(), b.parse::<f64>()) {
        n1.partial_cmp(&n2)
    } else {
        Some(a.cmp(b))
    }
}