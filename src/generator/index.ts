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
    const schemas = tools.findSQLSchemas(version, false)
    const treeSQL = {}
    for (const schema of schemas) {
        for (const filepath of tools.findSQLFiles({ version, schema })) {
            treeSQL[schema] = [...(treeSQL[schema] ?? []), ...tools.loadSQLFile(filepath, {})]
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

export interface WriteJsonOptions {
    dest: string
}

export function writeJSON(schema: DBSchema, options: Partial<WriteJsonOptions> = {}): void {
    const opts = {
        dest: "generated/db-schema.json",
        ...options
    }
    fs.mkdirSync(opts.dest.split("/").slice(0, -1).join("/"), { recursive: true })
    fs.writeFileSync(opts.dest, JSON.stringify(schema, null, 2))
}

export interface WriteLibraryOptions {
    dest: string
    libraryPath: string
    typesPath: string
}

export function writeLibrary(schema: DBSchema, options: Partial<WriteLibraryOptions> = {}): void {
    const opts: WriteLibraryOptions = {
        dest: "generated/index.ts",
        libraryPath: "../dist/lib.mjs",
        typesPath: "../dist/types/generator/lib/index.d.ts",
        ...options,
    }

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
        }).join("\n\n")
    
    const indexContent =
`
/* eslint-disable @typescript-eslint/prefer-namespace-keyword */
/* eslint-disable @typescript-eslint/no-namespace */
import type { kObject, ListOptions, ReadOptions, Options, Clause, IClient, DBSchema } from "${opts.typesPath}"
import { lib } from "${opts.libraryPath}"
export * as lib from "${opts.libraryPath}"

// -- INTERFACES --
export namespace schema {
${Object.keys(schema).map(schemaName => 
`    export namespace ${toPascalCase(schemaName)} {
${domainsTextG(schemaName)}
${enumsTextG(schemaName)}
${interfacesTextG(schemaName)}
    }`
).join("\n")}
}

const _info: DBSchema = ${JSON.stringify(schema)}

${Object.keys(schema).map(schemaName => apiTextG(schemaName)).join("\n\n")}

${Object.keys(schema).map(schemaName => `
class ${toPascalCase(schemaName)}Schema {
    constructor(
        private client: IClient,
    ) {}

    ${Object.keys(schema[schemaName].tables).map(tableName => `
    public get ${toPascalCase(tableName)}(): ${toPascalCase(tableName)}Table {
        return new ${toPascalCase(tableName)}Table(this.client);
    }
    `).join("\n")}
}`).join("\n\n")}

export class Api {
    constructor(
        private client: IClient,
    ) {}

    ${Object.keys(schema).map(schemaName => `
    public get ${toPascalCase(schemaName)}(): ${toPascalCase(schemaName)}Schema {
        return new ${toPascalCase(schemaName)}Schema(this.client);
    }
    `).join("\n\n")}
}

export const info = (): DBSchema => (${JSON.stringify(schema)})

export default { Api, info, ...lib }
`

    fs.mkdirSync(opts.dest.split("/").slice(0, -1).join("/"), { recursive: true })
    fs.writeFileSync(opts.dest, indexContent)
}
