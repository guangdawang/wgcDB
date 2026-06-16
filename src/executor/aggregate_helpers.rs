use crate::executor::projection::AggregateFunction;
use sqlparser::ast::*;

pub fn compute_agg(func: &AggregateFunction, rows: &[&Vec<String>], columns: &[String]) -> Result<String, String> {
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

pub fn evaluate_having(expr: &Expr, rows: &[&Vec<String>], columns: &[String]) -> Result<bool, String> {
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