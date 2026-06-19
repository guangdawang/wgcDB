use super::types::{Condition, extract_actual_column, compare_vals, compare_with_op};

/// 单表查询中的条件检查（不支持列对列）
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