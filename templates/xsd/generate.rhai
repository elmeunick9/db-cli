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

fn min_occurs(column) {
    if column["is_nullable"] {
        "0"
    } else {
        "1"
    }
}

fn main() {
    print("Registering Handlebars helpers...");
    register("xsd_type", "to_xsd_type");
    register("min_occurs", "min_occurs");

    print("Rendering catalog.xml...");
    save("catalog.xml", transform(context, load("catalog.xml.hbs")));

    let schemas = context["schemas"];
    let table_template = load("table.xsd.hbs");

    for schema_index in 0..len(schemas) {
        let schema = schemas[schema_index];
        let tables = schema["tables"];

        for table_index in 0..len(tables) {
            let table = tables[table_index];
            let output_context = #{};
            output_context["format"] = context["format"];
            output_context["version"] = context["version"];
            output_context["schemas"] = schemas;
            output_context["schema"] = schema;
            output_context["table"] = table;

            let output_path = schema["schema"] + "/" + table["name"] + ".xsd";
            print("Rendering table.xsd (" + table.name + ")...");
            save(output_path, transform(output_context, table_template));
        }
    }
}