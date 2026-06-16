use sqlparser::ast::*;
use std::cmp::Ordering;

/// WHERE 子句中的单个条件
pub struct Condition {
    pub col: String,
    pub op: BinaryOperator,
    pub val: String,
}

/// 递归提取 AND 连接的简单条件列表
pub fn extract_conditions(expr: &Expr) -> Result<Vec<Condition>, String> {
    match expr {
        Expr::BinaryOp { left, op, right } if *op == BinaryOperator::And => {
            let mut left_conds = extract_conditions(left)?;
            let mut right_conds = extract_conditions(right)?;
            left_conds.append(&mut right_conds);
            Ok(left_conds)
        }
        Expr::BinaryOp { left, op, right } => {
            let col = if let Expr::Identifier(ident) = &**left {
                ident.value.clone()
            } else {
                return Err("Left side of condition must be a column".into());
            };
            let val = match &**right {
                Expr::Value(ValueWithSpan { value, .. }) => match value {
                    Value::SingleQuotedString(s) => s.clone(),
                    Value::Number(n, _) => n.clone(),
                    Value::Boolean(b) => b.to_string(),
                    _ => return Err("Unsupported literal value".into()),
                },
                _ => return Err("Right side must be a literal".into()),
            };
            match op {
                BinaryOperator::Eq
                | BinaryOperator::NotEq
                | BinaryOperator::Lt
                | BinaryOperator::LtEq
                | BinaryOperator::Gt
                | BinaryOperator::GtEq => {}
                _ => return Err(format!("Unsupported operator: {}", op)),
            }
            Ok(vec![Condition {
                col,
                op: op.clone(),
                val,
            }])
        }
        _ => Err("Unsupported WHERE clause format".into()),
    }
}

/// 对单行检查一个条件是否成立
pub fn check_condition(
    row: &[String],
    columns: &[String],
    cond: &Condition,
) -> Result<bool, String> {
    let col_idx = columns
        .iter()
        .position(|c| c == &cond.col)
        .ok_or(format!("Column {} not found", cond.col))?;
    let row_val = &row[col_idx];
    let cmp = {
        if let (Ok(n1), Ok(n2)) = (row_val.parse::<f64>(), cond.val.parse::<f64>()) {
            n1.partial_cmp(&n2)
        } else {
            Some(row_val.cmp(&cond.val))
        }
    };
    match cond.op {
        BinaryOperator::Eq => Ok(cmp == Some(Ordering::Equal)),
        BinaryOperator::NotEq => Ok(cmp != Some(Ordering::Equal)),
        BinaryOperator::Lt => Ok(cmp == Some(Ordering::Less)),
        BinaryOperator::LtEq => Ok(cmp == Some(Ordering::Less) || cmp == Some(Ordering::Equal)),
        BinaryOperator::Gt => Ok(cmp == Some(Ordering::Greater)),
        BinaryOperator::GtEq => {
            Ok(cmp == Some(Ordering::Greater) || cmp == Some(Ordering::Equal))
        }
        _ => unreachable!(),
    }
}

/// 通用值比较：先尝试数字，再字符串（供 ORDER BY 等使用）
pub fn compare_values(a: &str, b: &str) -> Ordering {
    if let (Ok(n1), Ok(n2)) = (a.parse::<f64>(), b.parse::<f64>()) {
        n1.partial_cmp(&n2).unwrap_or(Ordering::Equal)
    } else {
        a.cmp(b)
    }
}