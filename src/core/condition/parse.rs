use super::types::Condition;
use sqlparser::ast::*;

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
            if let Ok(col_name) = extract_column_name(right) {
                Ok(vec![Condition { col, op: op_str, val: col_name, rhs_is_column: true }])
            } else if let Ok(lit_val) = extract_literal_value(right) {
                Ok(vec![Condition { col, op: op_str, val: lit_val, rhs_is_column: false }])
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