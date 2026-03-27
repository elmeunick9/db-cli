use crate::utils::inspect;
use std::fs as std_fs;
use std::fmt::Write;
use std::path::PathBuf;

pub fn write_json(output_dir: &PathBuf, schema_info: &inspect::SchemaInfo) -> Result<(), Box<dyn std::error::Error>> {
    let mut content = String::new();
    write!(&mut content, "{{\"schema\":{},\"tables\":[", json_string(&schema_info.name))?;

    let table_strings: Vec<String> = schema_info.tables.iter().map(format_table).collect();
    content.push_str(&table_strings.join(","));
    content.push_str("],\"enums\":[");

    let enum_strings: Vec<String> = schema_info.enums.iter().map(format_enum).collect();
    content.push_str(&enum_strings.join(","));
    content.push(']');
    content.push('}');

    let file_name = format!("{}.json", schema_info.name);
    let file_path = output_dir.join(file_name);
    std_fs::write(file_path, content)?;
    Ok(())
}

fn format_table(table: &inspect::TableInfo) -> String {
    let mut buf = String::new();
    let columns: Vec<String> = table.columns.iter().map(format_column).collect();
    let fks: Vec<String> = table.foreign_keys.iter().map(format_foreign_key).collect();

    write!(
        &mut buf,
        "{{\"name\":{},\"columns\":[{}],\"primary_key\":{},\"foreign_keys\":[{}]}}",
        json_string(&table.name),
        columns.join(","),
        format_string_array(&table.primary_key),
        fks.join(","),
    )
    .unwrap();

    buf
}

fn format_column(column: &inspect::ColumnInfo) -> String {
    let mut buf = String::new();
    write!(
        &mut buf,
        "{{\"name\":{},\"ordinal_position\":{},\"data_type\":{},\"udt_name\":{},\"is_nullable\":{},\"default\":",
        json_string(&column.name),
        column.ordinal_position,
        json_string(&column.data_type),
        json_string(&column.udt_name),
        column.is_nullable,
    )
    .unwrap();

    if let Some(value) = &column.default {
        write!(&mut buf, "{}", json_string(value)).unwrap();
    } else {
        buf.push_str("null");
    }

    buf.push('}');
    buf
}

fn format_foreign_key(fk: &inspect::ForeignKeyInfo) -> String {
    let mut buf = String::new();
    write!(
        &mut buf,
        "{{\"name\":{},\"columns\":{},\"referenced_schema\":{},\"referenced_table\":{},\"referenced_columns\":{}}}",
        json_string(&fk.name),
        format_string_array(&fk.columns),
        json_string(&fk.referenced_schema),
        json_string(&fk.referenced_table),
        format_string_array(&fk.referenced_columns),
    )
    .unwrap();
    buf
}

fn format_enum(en: &inspect::EnumInfo) -> String {
    let mut buf = String::new();
    write!(
        &mut buf,
        "{{\"name\":{},\"values\":{}}}",
        json_string(&en.name),
        format_string_array(&en.values),
    )
    .unwrap();
    buf
}

fn json_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{}\"", escaped)
}

fn format_string_array(values: &[String]) -> String {
    let mut parts = Vec::with_capacity(values.len());
    for value in values {
        parts.push(json_string(value));
    }
    format!("[{}]", parts.join(","))
}