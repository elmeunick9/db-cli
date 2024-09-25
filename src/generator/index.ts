import config from "../config"
import parser from '../parser'
import { DBSchema } from "../parser/enhance"
import { ASTNode } from "../parser/parser"
import { SchemaStore } from "../parser/prune"
import tools from "../tools"
import fs from 'fs'
import { buildApi, buildDomain, buildEnum, buildInterface } from "./builder"
import { toPascalCase } from "../tools/string_utils"

export interface AST {
    [schema: string]: ASTNode[]
}

export function ast(version = config.isDevelopment() ? 'next' : 'api'): AST {
    const schemas = tools.findSQLSchemas(version, false, false)
    const treeSQL = {}
    for (const schema of schemas) {
        for (const filepath of tools.findSQLFiles({ version, schema })) {
            treeSQL[schema] = [...(treeSQL[schema] ?? []), ...tools.loadSQLFile(filepath, {}, false)]
        }
    }

    const tree: AST = {}
    if (schemas.length == 0) {
        throw new Error("Empty DB!")
    }
    for (const schema of schemas) {
        tree[schema] = treeSQL[schema].map((sql: string) => {
            try {
                return parser.parse(sql)
            } catch (err) {
                console.error("!!!!!", err.message, err.ast, err.stack, sql)
                throw err
            }
        })
    }
    
    return tree
}

export function generateDBSchema(version = config.isDevelopment() ? 'next' : 'api'): DBSchema {
    const treeNode = ast(version)
    const tree: SchemaStore = {}
    for (const schema of Object.keys(treeNode)) {
        tree[schema] = treeNode[schema].map(stmt => parser.prune(stmt))
    }

    return parser.enhance(tree)
}

export function writeJSON(schema: DBSchema, dest = "generated/db-schema.json"): void {
    fs.mkdirSync(dest.split("/").slice(0, -1).join("/"), { recursive: true })
    fs.writeFileSync(dest, JSON.stringify(schema, null, 2))
}

export function writeLibrary(schema: DBSchema, dest = "dist/out/index.ts"): void {
    const interfacesTextG = (schemaName: string): string => Object.keys(schema[schemaName].tables)
        .map(tableName => {
            const table = schema[schemaName].tables[tableName]
            return buildInterface(schema[schemaName], table)
        }).join("\n")

    const domainsTextG = (schemaName: string): string => schema[schemaName].domains
        .map(domain => buildDomain(domain))
        .join("\n");

    const enumsTextG = (schemaName: string): string => schema[schemaName].enums
        .map(enumObj => buildEnum(enumObj))
        .join("\n")

    const apiTextG = (schemaName: string): string => Object.keys(schema[schemaName].tables)
        .map(tableName => {
            const table = schema[schemaName].tables[tableName]
            return buildApi(schemaName, table)
        }).join("\n")
    
    const indexContent =
`
/* eslint-disable @typescript-eslint/prefer-namespace-keyword */
/* eslint-disable @typescript-eslint/no-namespace */
import { type DBSchema } from "@hubbit86/db-cli/dist/types/parser/enhance"
import lib, { type kObject, type ListOptions, type ReadOptions, type Options, type Clause } from "@hubbit86/db-cli"
export * as lib from "@hubbit86/db-cli"

// -- INTERFACES --
export module schema {
${Object.keys(schema).map(schemaName => 
`    export module ${toPascalCase(schemaName)} {
${domainsTextG(schemaName)}
${enumsTextG(schemaName)}
${interfacesTextG(schemaName)}
    }`
).join("\n")}
}

const _info: DBSchema = ${JSON.stringify(schema)}
export module api {
${Object.keys(schema).map(schemaName => 
    `    export module ${toPascalCase(schemaName)} {
${apiTextG(schemaName)}
    }`
).join("\n")}
}

export const info = (): DBSchema => (${JSON.stringify(schema)})

export default { api, info, ...lib }
`

    fs.mkdirSync(dest.split("/").slice(0, -1).join("/"), { recursive: true })
    fs.writeFileSync(dest, indexContent)
}
