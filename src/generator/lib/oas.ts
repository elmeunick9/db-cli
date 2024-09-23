/* eslint-disable @typescript-eslint/no-explicit-any */
import { ColumnSchema } from "../../parser/prune"
import { Schema as DBSchema, TableSchemaEnhanced } from "../../parser/enhance";
import { SQLBooleanType, SQLIntegerType, SQLNumericType, SQLStringType, SQLTimeType } from "../builder";
import { toPascalCase } from "../../tools/string_utils";

    export interface ExternalDocumentation {
        description?: string;
        url: string;
      }

    export type SchemaType = 'array' | 'boolean' | 'integer' | 'number' | 'object' | 'string';

    export interface Reference {
        $ref: string;
    }

    export interface Discriminator {
        propertyName: string;
        mapping?: { [discriminatorValue: string]: string };
    }

    export interface XML {
        name?: string;
        namespace?: string;
        prefix?: string;
        attribute?: boolean;
        wrapped?: boolean;
    }

    export interface SchemaObject {
        title?: string;
        multipleOf?: number;
        maximum?: number;
        exclusiveMaximum?: boolean;
        minimum?: number;
        exclusiveMinimum?: boolean;
        maxLength?: number;
        minLength?: number;
        pattern?: string;
        maxItems?: number;
        minItems?: number;
        uniqueItems?: boolean;
        maxProperties?: number;
        minProperties?: number;
        required?: string[];
        enum?: any[];
        type?: SchemaType | SchemaType[];
        allOf?: (SchemaObject | Reference)[];
        oneOf?: (SchemaObject | Reference)[];
        anyOf?: (SchemaObject | Reference)[];
        not?: SchemaObject | Reference;
        items?: SchemaObject | Reference;
        properties?: { [propertyName: string]: SchemaObject | Reference };
        additionalProperties?: boolean | SchemaObject | Reference;
        description?: string;
        format?: string;
        default?: any;
        nullable?: boolean;
        discriminator?: Discriminator;
        readOnly?: boolean;
        writeOnly?: boolean;
        xml?: XML;
        externalDocs?: ExternalDocumentation;
        example?: any;
        deprecated?: boolean;
    }

interface NamedSchemaObject extends SchemaObject {
    name: string
}

function listToObject<T>(arr: NamedSchemaObject[]): { [name: string]: T } {
    if (!Array.isArray(arr)) throw new Error(`Not an array: ${arr}`)
    return arr.reduce((result, { name, ...moreProps }) => ({ ...result, [name]: moreProps }), {});
}

function SQLType2OAS(sql_type: string): SchemaObject {
    let obj: SchemaObject = {}
    let type: SchemaType = null
    
    if (SQLStringType.includes(sql_type))    type = "string"
    if (SQLTimeType.includes(sql_type))      type = "string"
    if (SQLIntegerType.includes(sql_type))   type = "integer"
    if (SQLNumericType.includes(sql_type))   type = "number"
    if (SQLBooleanType.includes(sql_type))   type = "boolean"

    obj = { ...obj, type}

    // Special
    if (["uuid"].includes(sql_type)) {
        obj.format = "uuid"
    }

    // Parameters
    if (sql_type.indexOf('(') > 0) {
        const prefix    = sql_type.split('(')[0].trim()
        const args      = sql_type.split('(')[1].split(')')[0].trim()
        if (["char", "varchar", "character", "character varying"].includes(prefix)) {
            obj = { ...obj, type: "string", maxLength: parseInt(args) }
        }
        if (["numeric", "decimal"].includes(prefix)) {
            obj = { ...obj, type: "string", format: prefix }
        }
    }

    // Date & Time
    if (SQLTimeType.includes(sql_type)) {
        if      (["date"].includes(sql_type))                       obj.format = "date"
        else if (["timestamp", "timestamptz"].includes(sql_type))   obj.format = "date-time"
        else                                                        obj.format = sql_type
    }

    return obj
}

function SQLColumn2OAS(column: ColumnSchema, schema: DBSchema): SchemaObject {
    let obj = SQLType2OAS(column.type)

    if (column.isNullable) {
        obj.nullable = true
    }

    // Is base type
    if (obj.type != null) return obj    

    // Alias
    if (schema != null) {
        const domainSchema    = schema.domains.find(x => x.name === column.type)
        const enumSchema      = schema.enums.find(x => x.name === column.type)
        if (domainSchema) {
            const alias_obj = SQLType2OAS(domainSchema.alias)
            obj = { ...obj, ...alias_obj }
            obj.format = column.type
            return obj
        }

        if (enumSchema) {
            obj.type = "string"
            obj.enum = column.isNullable ? [...enumSchema.values, null] : enumSchema.values
            return obj
        }        
    }

    return obj
}

export const oas = {
    tableToSchema: (
        table: TableSchemaEnhanced, 
        schema: DBSchema, 
        filter: "keys"|"data"|"all" = "all", 
        require: "normal"|"all"|"none" = "normal"
    ): SchemaObject => {
        const columns = table.columns.filter(x => {
            if (filter === "keys") return table.key.includes(x.name)
            if (filter === "data") return !table.key.includes(x.name)
            return true
        })
        const requiredColumns = table.columns.filter(x => {
            if (filter === "keys" && !table.key.includes(x.name)) {
                return false
            }
            if (require === "all")      return true
            if (require === "none")     return false
            return !x.isNullable && !x.hasDefault
        })
        return {
            type: "object",
            properties: listToObject(columns.map<NamedSchemaObject>((column: ColumnSchema) => ({
                name: column.name,
                ...SQLColumn2OAS(column, schema)
            }))),
            required: requiredColumns.map(x => x.name)
        }
    },
    schemaToSchema: (db_schema: DBSchema): { [key: string]: SchemaObject } => {
        const tables = Object.values(db_schema.tables)
        const keySchemas: NamedSchemaObject[] = tables.map(x => ({
            name: `DB_KEY_${toPascalCase(x.name)}`,
            ...oas.tableToSchema(x, db_schema, "keys")
        }))
        const dataSchemas: NamedSchemaObject[] = tables.map(x => ({
            name: `DB_POST_REQUEST_${toPascalCase(x.name)}`,
            ...oas.tableToSchema(x, db_schema, "data")
        }))
        const dataPartialSchemas: NamedSchemaObject[] = tables.map(x => ({
            name: `DB_PUT_REQUEST_${toPascalCase(x.name)}`,
            ...oas.tableToSchema(x, db_schema, "data", "none")
        }))
        const rowSchemas: NamedSchemaObject[] = tables.map(x => ({
            name: `DB_GET_RESPONSE_${toPascalCase(x.name)}`,
            type: "object",
            allOf: [
                { "$ref": `#/components/schemas/DB_KEY_${toPascalCase(x.name)}` },
                oas.tableToSchema(x, db_schema, "data", "all")
            ]
        }))
        const listSchemas: NamedSchemaObject[] = tables.map(x => ({
            name: `DB_LIST_RESPONSE_${toPascalCase(x.name)}`,
            type: "array",
            items: {
                allOf: [
                    { "$ref": `#/components/schemas/DB_KEY_${toPascalCase(x.name)}` },
                    oas.tableToSchema(x, db_schema, "data", "none"),
                ]
            }
        }))
        return listToObject([
            ...keySchemas,
            ...dataSchemas,
            ...dataPartialSchemas,
            ...rowSchemas,
            ...listSchemas,
        ])
    }
}

export default oas