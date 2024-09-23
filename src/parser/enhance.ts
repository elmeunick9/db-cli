import { ColumnSchema, DomainSchema, EnumSchema, SchemaAny, SchemaStore, TableSchema } from "./prune";

export interface TableSchemaEnhanced extends Omit<TableSchema, "inherits"> {
    path?: string
}

export interface DBSchema {
    [schema: string]: Schema
}

export interface Schema {
    tables: {
        [table: string]: TableSchemaEnhanced
    }
    domains: DomainSchema[]
    enums: EnumSchema[]
}

function isTable(obj: SchemaAny): obj is TableSchema {
    return obj != null && (obj as TableSchema).type == "table"
}

function isDomain(obj: SchemaAny): obj is DomainSchema {
    return obj != null && (obj as DomainSchema).type == "domain"
}

function isEnum(obj: SchemaAny): obj is EnumSchema {
    return obj != null && (obj as EnumSchema).type == "enum"
}

export function enhance(ast: SchemaStore): DBSchema {
    const arr2obj = <T>(arr: {name: string}[]): {[key: string]: T} => arr.reduce((obj, item) => (obj[item.name] = item, obj) , {})

    function extend(origin: TableSchema, from: string): ColumnSchema[] {
        if (!from) return origin.columns

        const [schema, table] = (from.at(0) == '"' && from.at(-1) == '"')
            ? from.slice(1, -1).split('"."')
            : from.split(".")
        
        const fromTable = ast[schema]
            .filter(table => isTable(table))
            .find((x: TableSchema) => x.name == table)
        
        return (fromTable as TableSchema).columns
    }

    const obj : DBSchema = {}
    for (const schema of Object.keys(ast)) {
        if (!ast[schema]) continue

        const tables = ast[schema]
            .filter(table => isTable(table))
            .map((table: TableSchema) => {
                if (table.inherits && table.inherits.length > 0) {
                    table.columns = [table.columns, ...table.inherits.map(x => extend(table, x))].flat()
                }
                delete table["inherits"]
                table["path"] = `"${schema}"."${table.name}"`
                return table
            })

        const domains: DomainSchema[] = ast[schema].filter(type => isDomain(type)) as DomainSchema[]
        const enums: EnumSchema[] = ast[schema].filter(type => isEnum(type)) as EnumSchema[]

        obj[schema] = {
            tables: arr2obj(tables),
            domains,
            enums
        }
    }

    return obj
}
