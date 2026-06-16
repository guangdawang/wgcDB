use crate::executor::condition::Condition;
use crate::executor::projection::{ProjectionInfo, ColumnExpr};
use std::cmp::Ordering;

pub fn cartesian_product<'a>(data: &[Vec<&'a Vec<String>>]) -> Vec<Vec<&'a Vec<String>>> {
    if data.is_empty() {
        return vec![vec![]];
    }
    let mut result = vec![vec![]];
    for list in data {
        let mut new_result = Vec::new();
        for existing in &result {
            for item in list {
                let mut new_comb = existing.clone();
                new_comb.push(*item);
                new_result.push(new_comb);
            }
        }
        result = new_result;
    }
    result
}

pub fn check_join_condition(
    rows: &[&Vec<String>],
    table_info: &[(Option<String>, Vec<String>)],
    conditions: &[Condition],
) -> Result<bool, String> {
    for cond in conditions {
        // 解析左侧列，获取表索引和列名
        let (left_tbl_idx, left_col_name) = resolve_column(&cond.col, table_info)?;
        let left_row = rows[left_tbl_idx];
        let left_columns = &table_info[left_tbl_idx].1;
        let left_col_idx = left_columns.iter().position(|c| *c == left_col_name).unwrap();
        let left_val = &left_row[left_col_idx];

        // 获取右侧值：可能是字面量或另一列
        let right_val = if cond.rhs_is_column {
            // 解析右侧列
            let (right_tbl_idx, right_col_name) = resolve_column(&cond.val, table_info)?;
            let right_row = rows[right_tbl_idx];
            let right_columns = &table_info[right_tbl_idx].1;
            let right_col_idx = right_columns.iter().position(|c| *c == right_col_name).unwrap();
            right_row[right_col_idx].clone()
        } else {
            cond.val.clone()
        };

        let cmp = {
            if let (Ok(n1), Ok(n2)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
                n1.partial_cmp(&n2)
            } else {
                Some(left_val.cmp(&right_val))
            }
        };
        let ok = match cond.op.as_str() {
            "="  => cmp == Some(Ordering::Equal),
            "!=" => cmp != Some(Ordering::Equal),
            "<"  => cmp == Some(Ordering::Less),
            "<=" => cmp == Some(Ordering::Less) || cmp == Some(Ordering::Equal),
            ">"  => cmp == Some(Ordering::Greater),
            ">=" => cmp == Some(Ordering::Greater) || cmp == Some(Ordering::Equal),
            _ => return Err("不支持的操作符".into()),
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

fn resolve_column(full_name: &str, table_info: &[(Option<String>, Vec<String>)]) -> Result<(usize, String), String> {
    if let Some(dot_pos) = full_name.find('.') {
        let prefix = &full_name[..dot_pos];
        let col = &full_name[dot_pos+1..];
        for (idx, (alias, _)) in table_info.iter().enumerate() {
            if let Some(a) = alias {
                if a == prefix { return Ok((idx, col.to_string())); }
            }
        }
        // 也检查真实表名
        for (_idx, (_alias, _)) in table_info.iter().enumerate() {
            // 原始表名在 TableRef 中没有直接存储，这里用 alias 近似（ alias 可能为 None 就是真实表名）
            // 简单处理：如果前缀等于表的 columns 的任意？不可行。
            // 我们可以在 table_info 中增加真实表名，但目前结构只有 alias 和 columns。
            // 对于没有别名的表，解析时 "u1.age" 要求前缀是别名。如果没有别名，无法匹配。
            // 这里暂时回退，要求使用时必须指定别名。
        }
        Err(format!("无法解析列 '{}'", full_name))
    } else {
        let mut found = None;
        for (idx, (_, cols)) in table_info.iter().enumerate() {
            if cols.contains(&full_name.to_string()) {
                if found.is_some() {
                    return Err(format!("列 '{}' 不明确", full_name));
                }
                found = Some(idx);
            }
        }
        found.map(|idx| (idx, full_name.to_string()))
             .ok_or_else(|| format!("列 '{}' 不存在", full_name))
    }
}

pub fn build_join_output(
    rows: &[&Vec<String>],
    table_info: &[(Option<String>, Vec<String>)],
    proj: &ProjectionInfo,
) -> Result<Vec<String>, String> {
    if proj.is_wildcard {
        let mut out = Vec::new();
        for row in rows {
            out.extend(row.iter().cloned());
        }
        return Ok(out);
    }
    let mut out = Vec::new();
    for expr in &proj.columns {
        match expr {
            ColumnExpr::Column(full_col) => {
                let (idx, col_name) = resolve_column(full_col, table_info)?;
                let cols = &table_info[idx].1;
                let pos = cols.iter().position(|c| *c == col_name).unwrap();
                out.push(rows[idx][pos].clone());
            }
            ColumnExpr::Aggregate(_) => return Err("JOIN 暂不支持聚合".into()),
        }
    }
    Ok(out)
}