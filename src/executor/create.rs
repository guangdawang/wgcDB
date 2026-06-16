// src/executor/create.rs
use super::ExecutionResult;
use crate::database::Database;
use sqlparser::ast::Statement;

pub fn execute_create(db: &mut Database, statement: &Statement) -> Result<ExecutionResult, String> {
    match statement {
        Statement::CreateTable(create) => {
            let col_names: Vec<String> = create
                .columns
                .iter()
                .map(|col| col.name.value.clone())
                .collect();
            db.add_table(&create.name.to_string(), col_names);
            Ok(ExecutionResult::CreateTable)
        }
        Statement::CreateIndex(create_index) => {
            let table_name = create_index.table_name.to_string();
            let column_name = create_index
                .columns
                .first()
                .ok_or("No column specified for index")?
                .column               // 字段是 column，不是 name
                .to_string();         // ObjectName 可以直接 to_string 得到列名
            let table = db.tables.get_mut(&table_name).ok_or("Table not found")?;
            table.build_index(&column_name)?;
            Ok(ExecutionResult::CreateIndex)
        }
        _ => Err("Unsupported CREATE statement".into()),
    }
}