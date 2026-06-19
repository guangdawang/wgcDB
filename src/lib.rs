// src/lib.rs
pub mod core;
pub mod database;
pub mod executor;

// 重新导出常用类型，方便测试引用
pub use database::Database;
pub use executor::{execute_sql, ExecutionResult};