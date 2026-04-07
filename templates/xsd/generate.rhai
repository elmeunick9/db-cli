fn to_xsd_type(column) {
    if column["domain_name"] != () {
        return "tns:" + column["domain_name"];
    }

    let udt_name = column["udt_name"];
    if udt_name == "bool" {
        return "xs:boolean";
    }
    if udt_name == "int2" || udt_name == "int4" {
        return "xs:int";
    }
    if udt_name == "int8" {
        return "xs:long";
    }
    if udt_name == "float4" {
        return "xs:float";
    }
    if udt_name == "float8" || udt_name == "numeric" {
        return "xs:decimal";
    }
    if udt_name == "date" {
        return "xs:date";
    }
    if udt_name == "timestamp" || udt_name == "timestamptz" {
        return "xs:dateTime";
    }
    if udt_name == "uuid" {
        return "xs:string";
    }
    if column["data_type"] == "USER-DEFINED" {
        return "tns:" + udt_name;
    }

    return "xs:string";
}

fn enrich_schema(schema) {
    let tables = schema["tables"];
    let table_count = len(tables);
    for table_index in 0..table_count {
        let table = tables[table_index];
        let table_name = table["name"];
        table["xsd_type_name"] = table_name + "Type";

        let columns = table["columns"];
        let column_count = len(columns);
        for column_index in 0..column_count {
            let column = columns[column_index];
            column["xsd_type"] = to_xsd_type(column);
            column["min_occurs"] = if column["is_nullable"] { "0" } else { "1" };
            columns[column_index] = column;
        }

        table["columns"] = columns;
        tables[table_index] = table;
    }
    schema["tables"] = tables;

    let domains = schema["domains"];
    let domain_count = len(domains);
    for domain_index in 0..domain_count {
        let domain = domains[domain_index];
        domain["xsd_base_type"] = to_xsd_type(domain);
        domains[domain_index] = domain;
    }
    schema["domains"] = domains;

    schema
}

fn build_outputs(schemas) {
    let outputs = [];

    let catalog = #{};
    catalog["path"] = "catalog.xml";
    catalog["template"] = "catalog.xml.hbs";
    outputs.push(catalog);

    let schema_count = len(schemas);
    for schema_index in 0..schema_count {
        let schema = schemas[schema_index];
        let tables = schema["tables"];

        for table_index in 0..len(tables) {
            let table = tables[table_index];
            let output = #{};
            output["path"] = schema["schema"] + "/" + table["name"] + ".xsd";
            output["template"] = "table.xsd.hbs";

            let output_context = #{};
            output_context["schema"] = schema;
            output_context["table"] = table;
            output["context"] = output_context;
            outputs.push(output);
        }
    }

    outputs
}

fn transform(context) {
    let schemas = context["schemas"];
    let schema_count = len(schemas);
    for schema_index in 0..schema_count {
        let schema = schemas[schema_index];
        schemas[schema_index] = enrich_schema(schema);
    }

    context["schemas"] = schemas;
    context["outputs"] = build_outputs(schemas);
    context
}