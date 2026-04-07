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

fn xsd_type(helper) {
    to_xsd_type(helper["args"][0])
}

fn min_occurs(helper) {
    if helper["args"][0]["is_nullable"] {
        "0"
    } else {
        "1"
    }
}

fn handlebars_helpers() {
    ["xsd_type", "min_occurs"]
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
    context["outputs"] = build_outputs(schemas);
    context
}