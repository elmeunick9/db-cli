import { unquote } from "../tools/string_utils"
import { ASTNode } from "./parser"
import { query, queryNode } from "./walk"

export interface ColumnSchema {
    name: string
    type: string
    isNullable: boolean
    hasDefault: boolean
}

export interface ReferenceSchema {
    source: {
        key: string[]
    }
    destination: {
        key: string[]
        table: string
    }
}

export interface TableSchema {
    type: "table"
    name: string
    columns: ColumnSchema[]
    key: string[]
    references: ReferenceSchema[]
    inherits: string[]
}

export interface DomainSchema {
    type: "domain"
    name: string
    alias: string
}

export interface EnumSchema {
    type: "enum"
    name: string
    values: string[]
}

export type SchemaAny = TableSchema | DomainSchema | EnumSchema | void

export interface SchemaStore {
    [schema: string]: SchemaAny[]
}

export function prune(ast: ASTNode): SchemaAny {

    function createTable(ast: ASTNode): TableSchema {
        const table: TableSchema = {
            type: "table",
            name: unquote(queryNode(ast, "table_name").children[0].value),
            columns: queryNode(ast, "column_set").children
                .filter(c => c.name == "column")
                .map(c => ({
                    name: unquote(queryNode(c, "identifier").value),
                    type: queryNode(c, "type")?.value ?? queryNode(c, "type_ref")?.value,
                    isNullable: queryNode(c, "null_constraint")?.value != "NOT NULL",
                    hasDefault: queryNode(c, "default_value")?.value != null
                })
            ),
            key: queryNode(ast, "primary_key")?.children.map(c => unquote(c.value)),
            references: query(ast, "foreign_key").map(s => ({
                source: {
                    key: queryNode(s, "source")?.children.map(c => unquote(c.value))
                },
                destination: {
                    key: queryNode(s, "dest")?.children.filter(c => c.name == "columns")[0].children.map(c => unquote(c.value)),
                    table: queryNode(s, "dest")?.children.filter(c => c.name == "identifier")[0].value         
                }
            })),
            inherits: queryNode(ast, "inherits")?.children.map(c => c.value)
        }

        // Second pass to make PK always NOT NULL
        for (const keyColumn of table.key) {
            table.columns.find(x => x.name === keyColumn).isNullable = false
        }
        return table
    }

    function createDomain(ast: ASTNode): DomainSchema {
        return {
            type: "domain",
            name: unquote(queryNode(ast, "identifier").value),
            alias: queryNode(ast, "type").value
        }
    }

    function createEnum(ast: ASTNode): EnumSchema {
        return {
            type: "enum",
            name: unquote(queryNode(ast, "identifier").value),
            values: queryNode(ast, "enum")?.children.map(x => unquote(x.value))
        }
    }

    const create_table = queryNode(ast, "create_table")
    if (create_table) {
        return createTable(create_table)
    }

    const create_domain = queryNode(ast, "create_domain")
    if (create_domain) {
        return createDomain(create_domain)
    }

    const create_type = queryNode(ast, "create_type")
    if (create_type && queryNode(ast, "enum") != null) {
        return createEnum(create_type)
    }
}
