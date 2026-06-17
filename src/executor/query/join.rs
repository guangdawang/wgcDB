use crate::database::Database;
use crate::executor::condition::Condition;
use crate::executor::projection::ProjectionInfo;
use std::cmp::Ordering;

#[derive(Clone)]
pub struct TableRef {
    pub name: String,
    pub alias: Option<String>,
}

pub fn parse_from_clause(from: &[sqlparser::ast::TableWithJoins]) -> Result<Vec<TableRef>, String> {
    let mut tables = Vec::new();
    for twj in from {
        match &twj.relation {
            sqlparser::ast::TableFactor::Table { name, alias, .. } => {
                let tbl_name = name.to_string();
                let alias_str = alias.as_ref().map(|a| a.name.value.clone());
                tables.push(TableRef { name: tbl_name, alias: alias_str });
            }
            _ => return Err("仅支持简单表名".into()),
        }
        if !twj.joins.is_empty() {
            return Err("暂不支持显式 JOIN，请使用逗号分隔表名".into());
        }
    }
    Ok(tables)
}

pub fn nested_loop_join(
    db: &Database,
    tables: Vec<TableRef>,
    conditions: &[Condition],
    proj: &ProjectionInfo,
) -> Result<(Vec<Vec<String>>, usize), String> {
    if tables.len() < 2 {
        return Err("JOIN 至少需要两张表".into());
    }

    let mut table_data = Vec::new();
    let mut table_cols = Vec::new();
    let mut total_scanned = 0;
    for tbl_ref in &tables {
        let table = db.tables.get(&tbl_ref.name).ok_or("表不存在")?;
        let rows: Vec<&Vec<String>> = table.rows.values().collect();
        total_scanned += rows.len();
        table_data.push(rows);
        table_cols.push((tbl_ref.alias.clone(), table.columns.clone()));
    }

    let cartesian = cartesian_product(&table_data);
    let mut result = Vec::new();
    for combined in cartesian {
        if check_join_condition(&combined, &table_cols, conditions)? {
            let out_row = build_join_output(&combined, &table_cols, proj)?;
            result.push(out_row);
        }
    }
    Ok((result, total_scanned))
}

// ---------- JOIN 辅助函数 ----------

fn cartesian_product<'a>(data: &[Vec<&'a Vec<String>>]) -> Vec<Vec<&'a Vec<String>>> {
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

fn check_join_condition(
    rows: &[&Vec<String>],
    table_info: &[(Option<String>, Vec<String>)],
    conditions: &[Condition],
) -> Result<bool, String> {
    for cond in conditions {
        let (left_tbl_idx, left_col_name) = resolve_column(&cond.col, table_info)?;
        let left_row = rows[left_tbl_idx];
        let left_columns = &table_info[left_tbl_idx].1;
        let left_col_idx = left_columns.iter().position(|c| *c == left_col_name).unwrap();
        let left_val = &left_row[left_col_idx];

        let right_val = if cond.rhs_is_column {
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

fn build_join_output(
    rows: &[&Vec<String>],
    table_info: &[(Option<String>, Vec<String>)],
    proj: &ProjectionInfo,
) -> Result<Vec<String>, String> {
    use crate::executor::projection::ColumnExpr;
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