use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub col: String,
    pub op: String,
    pub val: String,
    #[serde(default)]
    pub rhs_is_column: bool,
}

pub fn compare_with_op(op: &str, cmp: Ordering) -> bool {
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

pub fn compare_vals(a: &str, b: &str) -> Option<Ordering> {
    if let (Ok(n1), Ok(n2)) = (a.parse::<f64>(), b.parse::<f64>()) {
        n1.partial_cmp(&n2)
    } else {
        Some(a.cmp(b))
    }
}