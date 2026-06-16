use super::projection::ProjectionInfo;
use super::condition::Condition;
use crate::database::Database;
use super::join_helpers::{cartesian_product, check_join_condition, build_join_output};

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