use super::condition::{extract_conditions, Condition};
use super::ExecutionResult;
use super::aggregate;
use super::join;
use super::projection::{self, ColumnExpr};
use crate::database::Database;
use sqlparser::ast::*;

pub fn execute_select(db: &mut Database, query: &Query) -> Result<ExecutionResult, String> {
    let SetExpr::Select(select) = &*query.body else {
        return Err("Only simple SELECT".into());
    };
    let Select {
        projection,
        from,
        selection,
        group_by,
        having,
        ..
    } = select.as_ref();

    let order_by = &query.order_by;
    let limit_clause = &query.limit_clause;

    let proj_info = super::projection_parse::parse_projection(projection)?;
    let table_joins = join::parse_from_clause(from)?;

    let conditions: Vec<Condition> = if let Some(where_expr) = selection {
        extract_conditions(where_expr)?
    } else {
        vec![]
    };

    let has_aggregate = proj_info.has_aggregate
        || match group_by {
            GroupByExpr::Expressions(exprs, _) => !exprs.is_empty(),
            GroupByExpr::All(_) => true,
        };

    if has_aggregate {
        let table_ref = table_joins.into_iter().next().ok_or("聚合查询需要一张表")?;
        return aggregate::execute_aggregate(
            db,
            table_ref,
            &conditions,
            &proj_info,
            group_by,
            having.as_ref(),
            order_by,
            limit_clause,
        );
    }

    if table_joins.len() == 1 {
        let tbl = table_joins[0].clone();
        let table = db.tables.get(&tbl.name).ok_or("Table not found")?;
        let all_columns = table.columns.clone();

        let mut used_index = false;
        let scanned;
        let candidate: Vec<(u64, &Vec<String>)>;

        if conditions.len() == 1 && conditions[0].op == "=" {
            if let Some((rows, cnt)) = table.scan_with_index(&conditions[0]) {
                candidate = rows;
                scanned = cnt;
                used_index = true;
            } else {
                (candidate, scanned) = table.scan_with_condition(&conditions)?;
            }
        } else {
            (candidate, scanned) = table.scan_with_condition(&conditions)?;
        }

        let mut result_rows = Vec::new();
        for (_, row) in candidate {
            if proj_info.is_wildcard {
                result_rows.push(row.clone());
            } else {
                let mut proj_row = Vec::new();
                for expr in &proj_info.columns {
                    let val = match expr {
                        ColumnExpr::Column(col_name) => {
                            let idx = all_columns.iter().position(|c| *c == *col_name).unwrap();
                            row[idx].clone()
                        }
                        _ => "?".into(),
                    };
                    proj_row.push(val);
                }
                result_rows.push(proj_row);
            }
        }

        projection::apply_order_limit(&mut result_rows, &proj_info, &all_columns, order_by, limit_clause)?;

        for cond in &conditions {
            if cond.op == "=" {
                let hint = db.stats.record_and_check(&tbl.name, &cond.col, used_index);
                if let Some((tbl, col)) = hint {
                    if let Some(t) = db.tables.get_mut(&tbl) {
                        if !t.indexes.contains_key(&col) {
                            t.build_index(&col)?;
                            println!("⚡ 自动创建索引: {}.{}", tbl, col);
                        }
                    }
                }
            }
        }

        Ok(ExecutionResult::Select { rows: result_rows, scanned, used_index })
    } else {
        let (rows, scanned) = join::nested_loop_join(db, table_joins, &conditions, &proj_info)?;
        let mut result_rows = rows;
        projection::apply_order_limit(&mut result_rows, &proj_info, &[], order_by, limit_clause)?;
        Ok(ExecutionResult::Select { rows: result_rows, scanned, used_index: false })
    }
}