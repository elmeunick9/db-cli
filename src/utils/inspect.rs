use crate::utils::db::DbPool;
use sqlx::Row;
use std::io;

pub struct ColumnInfo {
    pub ordinal_position: i32,
    pub name: String,
    pub data_type: String,
    pub udt_name: String,
    pub is_nullable: bool,
    pub default: Option<String>,
}

pub struct TableInfo {
    pub schema: String,
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

pub struct SchemaInfo {
    pub name: String,
    pub tables: Vec<TableInfo>,
}

pub async fn inspect_schema(pool: &DbPool, schema: &str) -> Result<SchemaInfo, sqlx::Error> {
    let tables = list_tables(pool, schema).await?;
    Ok(SchemaInfo { name: schema.to_string(), tables })
}

pub async fn list_tables(pool: &DbPool, schema: &str) -> Result<Vec<TableInfo>, sqlx::Error> {
    match pool {
        DbPool::Postgres(p) => {
            let table_rows = sqlx::query(
                "SELECT table_name FROM information_schema.tables WHERE table_schema = $1 AND table_type = 'BASE TABLE' ORDER BY table_name",
            )
            .bind(schema)
            .fetch_all(p)
            .await?;
        
            let mut tables = Vec::with_capacity(table_rows.len());
            for row in table_rows {
                let table_name: String = row.get("table_name");
                let columns = list_columns(pool, schema, &table_name).await?;
                tables.push(TableInfo {
                    schema: schema.to_string(),
                    name: table_name,
                    columns,
                });
            }

            Ok(tables)
        }
        DbPool::MySql(_) => Err(sqlx::Error::Configuration(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "introspection currently only supports Postgres",
        )))),
        DbPool::DryRun => Err(sqlx::Error::Configuration(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "inspecting schema requires a real Postgres pool",
        )))),
    }
}

/// Gather column definitions for a single table.
pub async fn list_columns(
    pool: &DbPool,
    schema: &str,
    table: &str,
) -> Result<Vec<ColumnInfo>, sqlx::Error> {
    match pool {
        DbPool::Postgres(p) => {
            let column_rows = sqlx::query(
                "SELECT ordinal_position, column_name, data_type, udt_name, is_nullable, column_default FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
            )
            .bind(schema)
            .bind(table)
            .fetch_all(p)
            .await?;

            Ok(column_rows
                .into_iter()
                .map(|row| ColumnInfo {
                    ordinal_position: row.get("ordinal_position"),
                    name: row.get("column_name"),
                    data_type: row.get("data_type"),
                    udt_name: row.get("udt_name"),
                    is_nullable: row
                        .get::<String, _>("is_nullable")
                        .eq_ignore_ascii_case("YES"),
                    default: row.get("column_default"),
                })
                .collect())
        }
        DbPool::MySql(_) => Err(sqlx::Error::Configuration(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "introspection currently only supports Postgres",
        )))),
        DbPool::DryRun => Err(sqlx::Error::Configuration(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "inspecting schema requires a real Postgres pool",
        )))),
    }
}