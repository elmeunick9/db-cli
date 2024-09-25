import { Schema, TableSchemaEnhanced } from "../../parser/enhance";
import { DomainSchema, EnumSchema } from "../../parser/prune";
import { toPascalCase } from "../../tools/string_utils";

export const SQLIntegerType = ['smallint', 'bigint', 'bigserial', 'int2', 'int4', 'int8', 'integer', 'int', 'serial2', 'serial4', 'serial', 'smallserial']
export const SQLNumericType = ['double precision', 'float8', 'real', 'float4']
export const SQLStringType = ['uuid', 'text', 'char', 'varchar', 'character', 'character varying', 'json', 'jsonb']
export const SQLTimeType = ['timetz', 'timestamptz', 'timestamp', 'date', 'time']
export const SQLBooleanType = ['boolean', 'bool']

export function SQLType2JS(type: string, schema: Schema|null = null): string|null {
    if (SQLNumericType.includes(type) || SQLIntegerType.includes(type)) return "number"
    
    // Although pg-postgres will return a Date object, JSON.stringify() will 
    // automatically convert it to a string (since Date is not supported in JSON).
    // 
    // I hope these types are directly compatible with types in the front end,
    // if something as generic as Date isn't compatible we have a problem. 
    //
    // Additionally, notice that doing new Date(new Date()) works, so if someone
    // needing a date does this instead of (obj as Date) that is no problem! If
    // instead someone tries to do date.asISOString() on a string, we will have
    // a problem!
    //
    // Also, string is a more cross-compatible solution that Date()
    //
    // if (["date", "timestamptz", "timestamp"].includes(type)) return "Date"
    
    if (SQLStringType.includes(type) || SQLTimeType.includes(type)) return "string"
    if (SQLBooleanType.includes(type)) return "boolean"

    if (type.indexOf('(') > 0) {
        const supertype = type.split('(')[0]
        if (["char", "varchar", "character", "character varying"].includes(supertype)) return "string"
        if (["numeric", "decimal"].includes(supertype)) return "number"
    }

    if (schema != null) {
        if (schema.domains.map(x => x.name).includes(type)) return type
        if (schema.enums.map(x => x.name).includes(type)) return type
    }

    console.log("UNKNOWN TYPE:", type)
    return "unknown"
}

export function buildInterface(schema: Schema, table: TableSchemaEnhanced): string {
    const out: string[] = []
    const name = toPascalCase(table.name)

    out.push(`        export interface ${name + "Key"} {\n`)
    for (const column of table.columns.filter(x => table.key.includes(x.name))) {
        out.push(`            ${column.name}: ${SQLType2JS(column.type ?? "", schema)}\n`)
    }
    out.push("        }\n")

    out.push(`        export interface ${name + "Data"} {\n`)
    for (const column of table.columns.filter(x => !table.key.includes(x.name))) {
        out.push(`            ${column.name}${(column.isNullable || column.hasDefault) ? '?' : ''}: ${SQLType2JS(column.type ?? "", schema)}${column.isNullable ? "|null" : ""}\n`)
    }
    out.push("        }\n")

    out.push(`        export interface ${name} extends ${name + "Key"}, ${name + "Data"} {}\n`)

    out.push(`        export type ${name}Column = ${table.columns.map(x => `"${x.name}"`).join("|")}\n`)

    return out.join("")
}

export function buildDomain(domain: DomainSchema): string {
    return `        export type ${domain.name} = ${SQLType2JS(domain.alias)}`
}

export function buildEnum(enumObj: EnumSchema): string {
    return `        export type ${enumObj.name} = ${enumObj.values.map(x => `"${x}"`).join("|")}`
}

export function buildApi(schema: string, table: TableSchemaEnhanced): string {
    const sch = toPascalCase(schema)

    const lEmptyKey     = `{${table.key.map(x => `${x}: null`).join(', ')}}`

    const lSchema       = `schema.${sch}.${toPascalCase(table.name)}`
    const lSchemaKey    = `${lSchema}Key`
    const lSchemaData   = `${lSchema}Data`
    const lSchemaColumn = `${lSchema}Column`

    const lPromise      = `Promise<${lSchema}>`
    const lPromiseKey   = `Promise<${lSchemaKey}>`
    // const lPromiseData =`Promise<${lSchemaData}>`
    const lPromiseList  = `Promise<Partial<${lSchema}>[]>`

    const lPromiseReq   = `Promise<Required<${lSchema}>>`

    const lListImpl     = `lib.generic.list("${schema}", "${table.name}", options, values, _info)`
    const lCreateImpl   = `lib.generic.createUsingDefaultKey("${schema}", "${table.name}", ${lEmptyKey}, values) as unknown`
    const lReadImpl     = `lib.generic.readByKey("${schema}", "${table.name}", key, options, _info)`
    const lUpdateImpl   = `lib.generic.updateByKey("${schema}", "${table.name}", key, values, options)`
    const lIncrImpl     = `lib.generic.incrementByKey("${schema}", "${table.name}", key, values)`
    const lDeleteImpl   = `lib.generic.deleteByKey("${schema}", "${table.name}", key)`
    const lDeleteAImpl  = `lib.generic.deleteByFilter("${schema}", "${table.name}", filter, values)`
    const lPushImpl     = `lib.generic.push("${schema}", "${table.name}", values)`
    const lPopImpl      = `lib.generic.pop("${schema}", "${table.name}", key)`

    const listSign      = `list: async (options: Partial<ListOptions<${lSchemaColumn}>> = {}, values: kObject = {}): ${lPromiseList} => ${lListImpl} as ${lPromiseList}`
    const createSign    = `create: async (values: ${lSchemaData}): ${lPromiseKey} => ${lCreateImpl} as ${lPromiseKey}`
    const readSign      = `read: async (key: ${lSchemaKey}, options: Partial<ReadOptions<${lSchemaColumn}>> = {}): ${lPromiseReq} => ${lReadImpl} as ${lPromiseReq}`
    const updateSign    = `update: async (key: ${lSchemaKey}, values: Partial<${lSchemaData}>, options: Options = {}): Promise<void> => ${lUpdateImpl}`
    const incrSign      = `increment: async (key: ${lSchemaKey}, values: Partial<${lSchemaData}>): Promise<void> => ${lIncrImpl}`
    const deleteSign    = `delete: async (key: ${lSchemaKey}): Promise<void> => ${lDeleteImpl}`
    const deleteASign   = `deleteAll: async (filter: Clause<${lSchemaColumn}>, values: kObject = {}): Promise<void> => ${lDeleteAImpl}`
    const pushSign      = `push: async (values: ${lSchema}): Promise<void> => ${lPushImpl}`
    const popSign       = `pop: async (key: ${lSchemaKey}): ${lPromise} => ${lPopImpl} as ${lPromise}`

    const methods       = [listSign, createSign, readSign, updateSign, deleteSign, deleteASign, pushSign, popSign, incrSign]
    return `        export const ${toPascalCase(table.name)} = {${methods.map(x => `\n            ${x}`).join(",")}\n        }`
}