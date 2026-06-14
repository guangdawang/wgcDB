use super::ExecutionResult;
use crate::database::Database;
use sqlparser::ast::Statement;

pub fn execute_create(db: &mut Database, statement: &Statement) -> Result<ExecutionResult, String> {
    if let Statement::CreateTable(create) = statement {
        let col_names: Vec<String> = create
            .columns
            .iter()
            .map(|col| col.name.value.clone())
            .collect();
        db.add_table(&create.name.to_string(), col_names);
        Ok(ExecutionResult::CreateTable)
    } else {
        Err("Not a CREATE TABLE statement".into())
    }
}