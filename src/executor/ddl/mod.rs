// src/executor/ddl/mod.rs
use crate::database::wal::WalRecord;
use sqlparser::ast::*;

pub fn make_create_record(statement: &Statement) -> Result<WalRecord, String> {
    match statement {
        Statement::CreateTable(create) => {
            let col_names: Vec<String> = create.columns.iter().map(|col| col.name.value.clone()).collect();
            let name = create.name.to_string();
            Ok(WalRecord::CreateTable { name, columns: col_names })
        }
        Statement::CreateIndex(create_index) => {
            let index_name = create_index.name
                .as_ref()
                .expect("CREATE INDEX 必须有索引名")
                .to_string();
            let table_name = create_index.table_name.to_string();
            let column_name = create_index.columns.first()
                .ok_or("No column specified for index")?
                .column.to_string();
            Ok(WalRecord::CreateIndex {
                table: table_name,
                index_name,
                column: column_name,
            })
        }
        _ => Err("Unsupported CREATE statement".into()),
    }
}

pub fn make_drop_index_record(
    names: &[ObjectName],
    table_name: &Option<ObjectName>,
) -> Result<WalRecord, String> {
    let idx_name = names.first().ok_or("索引名缺失")?.to_string();
    let tbl_name = table_name.as_ref().ok_or("DROP INDEX 需要 ON table_name")?.to_string();
    Ok(WalRecord::DropIndex { table: tbl_name, index_name: idx_name })
}