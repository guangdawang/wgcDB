use super::condition::compare_values;
use sqlparser::ast::*;

pub struct ProjectionInfo {
    pub is_wildcard: bool,
    pub columns: Vec<ColumnExpr>,
    pub has_aggregate: bool,
}

pub enum ColumnExpr {
    Column(String),
    Aggregate(AggregateFunction),
}

pub enum AggregateFunction {
    Count(String),
    Sum(String),
    Avg(String),
}

pub fn apply_order_limit(
    rows: &mut Vec<Vec<String>>,
    proj: &ProjectionInfo,
    all_columns: &[String],
    order_by: &Option<OrderBy>,
    limit_clause: &Option<LimitClause>,
) -> Result<(), String> {
    if let Some(order_by) = order_by {
        let exprs = match &order_by.kind {
            OrderByKind::Expressions(exprs) => exprs.as_slice(),
            OrderByKind::All(_) => &[],
        };
        if exprs.len() > 1 {
            return Err("仅支持单列排序".into());
        }
        if let Some(ob) = exprs.first() {
            let col_name = match &ob.expr {
                Expr::Identifier(ident) => ident.value.clone(),
                _ => return Err("ORDER BY 仅支持列名".into()),
            };
            let ascending = ob.options.asc.unwrap_or(true);
            if proj.is_wildcard {
                let col_idx = all_columns.iter().position(|c| c == &col_name).unwrap_or(0);
                rows.sort_by(|a, b| {
                    let cmp = compare_values(&a[col_idx], &b[col_idx]);
                    if ascending { cmp } else { cmp.reverse() }
                });
            } else {
                let pos = proj.columns.iter().position(|c| matches!(c, ColumnExpr::Column(s) if s == &col_name));
                let empty_a = String::new();
                let empty_b = String::new();
                rows.sort_by(|a, b| {
                    let va = pos.and_then(|p| a.get(p)).unwrap_or(&empty_a);
                    let vb = pos.and_then(|p| b.get(p)).unwrap_or(&empty_b);
                    let cmp = compare_values(va, vb);
                    if ascending { cmp } else { cmp.reverse() }
                });
            }
        }
    }
    if let Some(limit_clause) = limit_clause {
        let limit_num = match limit_clause {
            LimitClause::LimitOffset { limit, .. } => {
                if let Some(Expr::Value(ValueWithSpan { value: Value::Number(n, _), .. })) = limit {
                    n.parse::<usize>().map_err(|_| "无效 LIMIT".to_string())?
                } else {
                    return Err("LIMIT 必须是数字".into());
                }
            }
            LimitClause::OffsetCommaLimit { limit, .. } => {
                if let Expr::Value(ValueWithSpan { value: Value::Number(n, _), .. }) = limit {
                    n.parse::<usize>().map_err(|_| "无效 LIMIT".to_string())?
                } else {
                    return Err("LIMIT 必须是数字".into());
                }
            }
        };
        rows.truncate(limit_num);
    }
    Ok(())
}