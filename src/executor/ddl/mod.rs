use crate::database::Database;
use crate::database::wal::WalRecord;
use crate::executor::ExecutionResult;
use sqlparser::ast::*;

pub fn execute_create(db: &mut Database, statement: &Statement) -> Result<(ExecutionResult, Option<WalRecord>), String> {
    match statement {
        Statement::CreateTable(create) => {
            let col_names: Vec<String> = create.columns.iter().map(|col| col.name.value.clone()).collect();
            let name = create.name.to_string();
            db.add_table(&name, col_names.clone());
            let wal = Some(WalRecord::CreateTable { name, columns: col_names });
            Ok((ExecutionResult::CreateTable, wal))
        }
        Statement::CreateIndex(create_index) => {
            let table_name = create_index.table_name.to_string();
            let column_name = create_index.columns.first()
                .ok_or("No column specified for index")?
                .column.to_string();
            let table = db.tables.get_mut(&table_name).ok_or("Table not found")?;
            table.build_index(&column_name)?;
            let wal = Some(WalRecord::CreateIndex { table: table_name, column: column_name });
            Ok((ExecutionResult::CreateIndex, wal))
        }
        _ => Err("Unsupported CREATE statement".into()),
    }
}

pub fn execute_drop_index(
    db: &mut Database,
    names: &[ObjectName],
    table_name: &Option<ObjectName>,
) -> Result<(ExecutionResult, Option<WalRecord>), String> {
    let idx_name = names.first().ok_or("索引名缺失")?.to_string();
    let tbl_name = table_name.as_ref().ok_or("DROP INDEX 需要 ON table_name")?.to_string();

    let table = db.tables.get_mut(&tbl_name).ok_or("Table not found")?;
    table.drop_index(&idx_name);
    let wal = Some(WalRecord::DropIndex { table: tbl_name, column: idx_name });
    Ok((ExecutionResult::DropIndex, wal))
}