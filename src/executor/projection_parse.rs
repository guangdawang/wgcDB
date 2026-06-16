use crate::executor::projection::{ProjectionInfo, ColumnExpr, AggregateFunction};
use sqlparser::ast::*;

pub fn parse_projection(items: &[SelectItem]) -> Result<ProjectionInfo, String> {
    let mut cols = Vec::new();
    let mut has_agg = false;
    let mut is_wildcard = false;
    for item in items {
        match item {
            SelectItem::Wildcard(_) => {
                is_wildcard = true;
                break;
            }
            SelectItem::UnnamedExpr(expr) => match expr {
                Expr::Identifier(ident) => {
                    cols.push(ColumnExpr::Column(ident.value.clone()));
                }
                Expr::CompoundIdentifier(parts) => {
                    let name = parts.iter()
                        .map(|p| p.value.clone())
                        .collect::<Vec<_>>()
                        .join(".");
                    cols.push(ColumnExpr::Column(name));
                }
                Expr::Function(func) => {
                    let name = func.name.to_string().to_uppercase();
                    let args = &func.args;
                    match name.as_str() {
                        "COUNT" => {
                            let arg = if let FunctionArguments::List(list) = args {
                                if list.args.is_empty() {
                                    "*".into()
                                } else {
                                    extract_single_arg(&list.args)?
                                }
                            } else {
                                "*".into()
                            };
                            cols.push(ColumnExpr::Aggregate(AggregateFunction::Count(arg)));
                            has_agg = true;
                        }
                        "SUM" => {
                            let arg = if let FunctionArguments::List(list) = args {
                                extract_single_arg(&list.args)?
                            } else {
                                return Err("SUM 需要参数".into());
                            };
                            cols.push(ColumnExpr::Aggregate(AggregateFunction::Sum(arg)));
                            has_agg = true;
                        }
                        "AVG" => {
                            let arg = if let FunctionArguments::List(list) = args {
                                extract_single_arg(&list.args)?
                            } else {
                                return Err("AVG 需要参数".into());
                            };
                            cols.push(ColumnExpr::Aggregate(AggregateFunction::Avg(arg)));
                            has_agg = true;
                        }
                        _ => return Err(format!("不支持的函数: {}", name)),
                    }
                }
                _ => return Err("SELECT 中只能出现列名、* 或聚合函数".into()),
            },
            _ => return Err("不支持的 SELECT 项".into()),
        }
    }
    Ok(ProjectionInfo { is_wildcard, columns: cols, has_aggregate: has_agg })
}

fn extract_single_arg(args: &[FunctionArg]) -> Result<String, String> {
    if args.len() != 1 {
        return Err("聚合函数只接受一个参数".into());
    }
    match &args[0] {
        FunctionArg::Unnamed(arg_expr) => match arg_expr {
            FunctionArgExpr::Expr(Expr::Identifier(ident)) => Ok(ident.value.clone()),
            FunctionArgExpr::Wildcard => Ok("*".into()),
            _ => Err("参数格式错误".into()),
        },
        FunctionArg::Named { arg, .. } => match arg {
            FunctionArgExpr::Expr(Expr::Identifier(ident)) => Ok(ident.value.clone()),
            _ => Err("参数格式错误".into()),
        },
        FunctionArg::ExprNamed { .. } => Err("不支持的参数格式".into()),
    }
}