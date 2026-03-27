use crate::utils::db::DbPool;
use sqlx::Row;
use std::collections::HashMap;
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
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
}

pub struct SchemaInfo {
    pub name: String,
    pub tables: Vec<TableInfo>,
    pub enums: Vec<EnumInfo>,
}

pub struct ForeignKeyInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
}

pub struct EnumInfo {
    pub name: String,
    pub values: Vec<String>,
}

pub async fn inspect_schema(pool: &DbPool, schema: &str) -> Result<SchemaInfo, sqlx::Error> {
    let tables = list_tables(pool, schema).await?;
    let enums = list_enums(pool, schema).await?;
    Ok(SchemaInfo { name: schema.to_string(), tables, enums })
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

            let mut primary_keys: HashMap<String, Vec<String>> = HashMap::new();
            let pk_rows = sqlx::query(
                "SELECT kcu.table_name, kcu.column_name FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name AND tc.constraint_schema = kcu.constraint_schema WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_schema = $1 ORDER BY kcu.table_name, kcu.ordinal_position",
            )
            .bind(schema)
            .fetch_all(p)
            .await?;
            for row in pk_rows {
                let table_name: String = row.get("table_name");
                let column_name: String = row.get("column_name");
                primary_keys.entry(table_name).or_default().push(column_name);
            }

            let mut foreign_keys: HashMap<String, Vec<ForeignKeyInfo>> = HashMap::new();
            let fk_rows = sqlx::query(
                "SELECT kcu.table_name, tc.constraint_name, kcu.column_name, ccu.table_schema AS referenced_schema, ccu.table_name AS referenced_table, ccu.column_name AS referenced_column FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name AND tc.constraint_schema = kcu.constraint_schema JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name AND tc.constraint_schema = ccu.constraint_schema WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = $1 ORDER BY kcu.table_name, tc.constraint_name, kcu.ordinal_position",
            )
            .bind(schema)
            .fetch_all(p)
            .await?;

            let mut fk_builders: HashMap<(String, String), (Vec<String>, Vec<String>, String, String)> = HashMap::new();
            for row in fk_rows {
                let table_name: String = row.get("table_name");
                let constraint_name: String = row.get("constraint_name");
                let column_name: String = row.get("column_name");
                let ref_schema: String = row.get("referenced_schema");
                let ref_table: String = row.get("referenced_table");
                let ref_column: String = row.get("referenced_column");
                let key = (table_name.clone(), constraint_name.clone());
                let entry = fk_builders.entry(key).or_insert_with(|| (Vec::new(), Vec::new(), ref_schema.clone(), ref_table.clone()));
                entry.0.push(column_name);
                entry.1.push(ref_column);
            }
            for ((table_name, constraint_name), (columns, ref_columns, ref_schema, ref_table)) in fk_builders {
                foreign_keys.entry(table_name).or_default().push(ForeignKeyInfo {
                    name: constraint_name,
                    columns,
                    referenced_schema: ref_schema,
                    referenced_table: ref_table,
                    referenced_columns: ref_columns,
                });
            }
        
            let mut tables = Vec::with_capacity(table_rows.len());
            for row in table_rows {
                let table_name: String = row.get("table_name");
                let columns = list_columns(pool, schema, &table_name).await?;
                tables.push(TableInfo {
                    name: table_name.clone(),
                    columns,
                    primary_key: primary_keys.remove(&table_name).unwrap_or_default(),
                    foreign_keys: foreign_keys.remove(&table_name).unwrap_or_default(),
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

pub async fn list_enums(pool: &DbPool, schema: &str) -> Result<Vec<EnumInfo>, sqlx::Error> {
    match pool {
        DbPool::Postgres(p) => {
            let rows = sqlx::query("SELECT t.typname, array_agg(e.enumlabel ORDER BY e.enumsortorder) AS values FROM pg_type t JOIN pg_enum e ON t.oid = e.enumtypid JOIN pg_namespace n ON n.oid = t.typnamespace WHERE n.nspname = $1 GROUP BY t.typname ORDER BY t.typname")
                .bind(schema)
                .fetch_all(p)
                .await?;

            Ok(rows
                .into_iter()
                .map(|row| EnumInfo { name: row.get("typname"), values: row.get("values") })
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