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
    content.push_str("],\"domains\":[");

    let domain_strings: Vec<String> = schema_info.domains.iter().map(format_domain).collect();
    content.push_str(&domain_strings.join(","));
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
        "{{\"name\":{},\"ordinal_position\":{},\"data_type\":{},\"udt_name\":{},\"domain_schema\":{},\"domain_name\":{},\"is_nullable\":{},\"default\":",
        json_string(&column.name),
        column.ordinal_position,
        json_string(&column.data_type),
        json_string(&column.udt_name),
        format_optional_string(column.domain_schema.as_deref()),
        format_optional_string(column.domain_name.as_deref()),
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

fn format_domain(domain: &inspect::DomainInfo) -> String {
    let mut buf = String::new();
    let constraints: Vec<String> = domain
        .check_constraints
        .iter()
        .map(format_domain_constraint)
        .collect();
    write!(
        &mut buf,
        "{{\"name\":{},\"data_type\":{},\"udt_name\":{},\"is_nullable\":{},\"default\":{},\"check_constraints\":[{}]}}",
        json_string(&domain.name),
        json_string(&domain.data_type),
        json_string(&domain.udt_name),
        domain.is_nullable,
        format_optional_string(domain.default.as_deref()),
        constraints.join(","),
    )
    .unwrap();
    buf
}

fn format_domain_constraint(constraint: &inspect::DomainConstraintInfo) -> String {
    let mut buf = String::new();
    write!(
        &mut buf,
        "{{\"name\":{},\"definition\":{}}}",
        json_string(&constraint.name),
        json_string(&constraint.definition),
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

fn format_optional_string(value: Option<&str>) -> String {
    match value {
        Some(value) => json_string(value),
        None => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::write_json;
    use crate::utils::inspect::{
        ColumnInfo, DomainConstraintInfo, DomainInfo, EnumInfo, ForeignKeyInfo, SchemaInfo,
        TableInfo,
    };
    use rand::random;
    use std::fs;

    #[test]
    fn writes_domains_and_column_domain_references() {
        let temp_dir = std::env::temp_dir().join(format!("db-cli-json-test-{}", random::<u64>()));
        fs::create_dir_all(&temp_dir).unwrap();

        let schema_info = SchemaInfo {
            name: "auth".to_string(),
            tables: vec![TableInfo {
                name: "users".to_string(),
                columns: vec![ColumnInfo {
                    ordinal_position: 1,
                    name: "email".to_string(),
                    data_type: "text".to_string(),
                    udt_name: "text".to_string(),
                    domain_schema: Some("auth".to_string()),
                    domain_name: Some("email_address".to_string()),
                    is_nullable: false,
                    default: None,
                }],
                primary_key: vec!["email".to_string()],
                foreign_keys: vec![ForeignKeyInfo {
                    name: "users_org_id_fkey".to_string(),
                    columns: vec!["org_id".to_string()],
                    referenced_schema: "auth".to_string(),
                    referenced_table: "orgs".to_string(),
                    referenced_columns: vec!["id".to_string()],
                }],
            }],
            enums: vec![EnumInfo {
                name: "user_role".to_string(),
                values: vec!["admin".to_string(), "user".to_string()],
            }],
            domains: vec![DomainInfo {
                name: "email_address".to_string(),
                data_type: "text".to_string(),
                udt_name: "text".to_string(),
                is_nullable: false,
                default: None,
                check_constraints: vec![DomainConstraintInfo {
                    name: "email_address_check".to_string(),
                    definition: "CHECK (VALUE <> ''::text)".to_string(),
                }],
            }],
        };

        write_json(&temp_dir, &schema_info).unwrap();

        let content = fs::read_to_string(temp_dir.join("auth.json")).unwrap();
        assert!(content.contains("\"domain_schema\":\"auth\""));
        assert!(content.contains("\"domain_name\":\"email_address\""));
        assert!(content.contains("\"domains\":[{"));
        assert!(content.contains("\"check_constraints\":[{\"name\":\"email_address_check\""));

        fs::remove_dir_all(temp_dir).unwrap();
    }
}