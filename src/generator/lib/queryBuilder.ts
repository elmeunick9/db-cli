import config from "../../config"
import { DBSchema, TableSchemaEnhanced } from "../../parser/enhance"
import { prefixSchema, unquote } from "../../tools/string_utils"


export class Raw {
    constructor(public str: string) { }

    toString(): string {
        return this.str
    }
}

export type BaseAtomicType = string | number | boolean | null
export type BaseType = BaseAtomicType | object | BaseType[]
export interface SQLQuery {
    text: string
    values: BaseType[]
}

export interface ClauseBinaryOp<T> {
    column: T
    operator: "=" | "LIKE" | "ILIKE" | ">" | "<" | ">=" | "<=" | "!=" | "<>" | "IN" | "= ANY"
    key?: string
    value?: BaseAtomicType
}

export interface ClauseIsNull<T> {
    column: T
    operator: "IS NULL" | "IS NOT NULL"
}

export interface ClauseLogical<T> {
    operator: "AND" | "OR"
    clauses: Clause<T>[]
}

export interface Expand<T extends string> {
    key: T[]

    // Not sure how to get expanded types here.
    expand?: Expand<string>[]
    columns?: string[]
}

export interface ExpandedExpression {
    text: string
    columns: ExpandedColumnAlias[]
}

export interface ExpandedColumnAlias {
    column: string
    alias: string
}
// type Extend = { table: string, columns: string[], filter?: Clause<unknown> }

export type Clause<T> = ClauseLogical<T> | ClauseBinaryOp<T> | ClauseIsNull<T>

export function valueLiteral(value: BaseAtomicType): string {
    if (typeof value === "string") return `'${value}'`
    if (typeof value === "number") return `${value}`
    if (typeof value === "boolean") return `${value ? "TRUE" : "FALSE"}`
    throw new Error(`Invalid type: ${value}`)
}

export function format(sql: string, params: { [key: string]: BaseType|undefined }): SQLQuery {
    if (!params) return { text: sql, values: [] }
    const keys = Object.keys(removeUndefined(params))
    const text = sql.replace(/\$.*?\$/gmu, (x) => `$${keys.indexOf(x.slice(1, -1)) + 1}`)
    const values = keys.map(k => params[k])
    return { text, values }
}

export function removeUndefined(values: object | unknown[]): object | unknown[] {
    if (Array.isArray(values)) return values.filter(x => x !== undefined)

    for (const key of Object.keys(values)) {
        if (values[key] === undefined) delete values[key]
    }

    return values
}

export function prefixColumns(prefix: string, columns: (string | Raw)[]): (string | Raw)[] {
    return columns.map(validateColumn).map(x => {
        if (x instanceof Raw) return `${prefix}${x}`

        validateColumn(x)
        return new Raw(`"${prefix}"."${x}"`)
    })
}

export function validateColumn(k: string | Raw): string | Raw {
    if (k instanceof Raw) return k
    if ((/^[a-z][_a-z0-9]*$/.test(k))) return k
    else throw new Error(`Invalid column name (${k})`)
}

function matchingStringList(arr1: string[], arr2: string[]): boolean {
    return arr1.length === arr2.length && arr1.every(item => arr2.includes(item));
}

export const qb = {

    columnList: (columns: object | unknown[], prefix = ""): string => {
        if (!Array.isArray(columns)) columns = Object.keys(removeUndefined(columns))
        return (columns as unknown[]).flat().map(validateColumn).map(x => {
            if (x instanceof Raw) return x
            else return `${prefix}"${x}"`
        }).join(', ')
    },

    valueList: (columns: object | unknown[]): string => {
        if (!Array.isArray(columns)) columns = Object.keys(removeUndefined(columns))
        return (columns as unknown[]).flat().map(validateColumn).map(x => {
            if (x instanceof Raw) return x
            else return `$${x}$`
        }).join(', ')
    },

    assignList: (values: { [key: string]: unknown }): string => {
        if (Object.entries(values).length === 0) throw "No columns declared"
        return Object.keys(removeUndefined(values)).map(validateColumn).map(k => {
            return `"${k}" = $${k}$`
        }).join(', ')
    },

    incrementList: (values: { [key: string]: unknown }): string => {
        if (Object.entries(values).length === 0) throw "No columns declared"
        return Object.keys(removeUndefined(values)).map(validateColumn).map(k => {
            return `"${k}" = "${k}" + $${k}$`
        }).join(', ')
    },

    whereMatchList: (values: { [key: string]: unknown }, prefix = ""): string => {
        if (Object.entries(values).length === 0) throw "No columns declared"
        return Object.keys(values).map(validateColumn).map(k => {
            return `${prefix}"${k}" = $${k}$`
        }).join(' AND ')
    },

    whereClause: <T>(clause: Clause<T>): string => {
        if (["IS NULL", "IS NOT NULL"].includes(clause.operator)) {
            const c = clause as ClauseIsNull<T>
            return `"${c.column}" ${clause.operator}`
        }
        if (["=", "LIKE", "ILIKE", ">", "<", ">=", "<=", "!=", "<>", "IN", "= ANY"].includes(clause.operator)) {
            const c = clause as ClauseBinaryOp<T>
            if (c.value !== undefined) {
                return `"${c.column}" ${clause.operator} ${valueLiteral(c.value)}`
            }
            if (c.key != null) {
                if (clause.operator == "= ANY") {
                    return `"${c.column}" ${clause.operator} ($${c.key}$)`
                }
                return `"${c.column}" ${clause.operator} $${c.key}$`
            }
            throw new Error(`Filter with binary clause and no value or key! Expr: ${JSON.stringify(c)}`)
        }
        if (["AND", "OR"].includes(clause.operator)) {
            const c = clause as ClauseLogical<T>
            return c.clauses.map(x => qb.whereClause(x)).join(` ${c.operator} `)
        }
        throw new Error("Invalid Operator!")
    },

    expand: <T extends string>(schema: DBSchema, schemaName: string, tableName: string, ex: Expand<T>, alias?: string, tableNameAlias?: string, parentAliasPrefix?: string): ExpandedExpression => {
        // eslint-disable-next-line @typescript-eslint/explicit-function-return-type
        const getTablePath = (input: string) => input.includes('.')
            ? { schemaName: unquote(input.split('.')[0]), tableName: unquote(input.split('.')[1]) }
            : { schemaName: schemaName, tableName: unquote(input) }

        const table: TableSchemaEnhanced = schema[schemaName].tables[tableName]
        const dest = table.references.find(x => matchingStringList(x.source.key, ex.key))
        const destPath = getTablePath(dest.destination.table)
        const destTable = schema[destPath.schemaName].tables[destPath.tableName]
        const destColumns = destTable.columns
            .filter(x => !ex.columns || ex.columns.includes(x.name))
            .map(x => ({
                column: `"${alias ?? destPath.tableName}"."${x.name}"`,
                alias: `${parentAliasPrefix ?? ""}${dest.source.key.join("__")}.${x.name}`
            }))

        const joinCheckDestTable = dest.destination.table.includes('"."') 
            ? `"${prefixSchema(dest.destination.table.split('"."')[0].slice(1))}"."${dest.destination.table.split('"."')[1]}`
            : dest.destination.table
        const joinCheck = dest.source.key.map(x => `"${tableNameAlias || tableName}"."${x}" = ${`"${alias}"` ?? joinCheckDestTable}."${dest.destination.key}"`).join(" AND ")
        const join = `LEFT JOIN "${config.db.prefix}${destPath.schemaName}"."${destPath.tableName}" ${alias ? `AS "${alias}"` : ""} ON ${joinCheck}`
        const inner = ex.expand?.map((x, i) => qb.expand(
            schema, destPath.schemaName, 
            destPath.tableName, 
            x, 
            alias ? `${alias}_${i}` : undefined,
            alias,
            `${dest.source.key.join("__")}.`
        ))
        const inText = inner?.map(x => x.text).join("\n") ?? ""
        const inColumns = inner?.reduce((a, x) => [...a, ...x.columns], []) ?? []
        return {
            text: [join, inText].join("\n"),
            columns: [...destColumns, ...inColumns]
        }
    },

    nest: (input: Record<string, unknown>): Record<string, unknown> => {
        const result: Record<string, unknown> = {};
        const keysToRemove: string[] = [];
        const keysToNest: Set<string> = new Set();

        for (const key in input) {
            if (key.includes('.')) {
                const parts = key.split(".")
                const prefix = parts[0]
                const suffix = parts.slice(1).join(".")

                if (result[prefix] === undefined || result[prefix] === null || typeof result[prefix] !== "object") {
                    result[prefix] = {};
                }
                
                result[prefix][suffix] = input[key];

                keysToNest.add(prefix);
                if (prefix.includes('__')) {
                    const parts = prefix.split('__');
                    keysToRemove.push(...parts);
                }
            } else {
                if (result[key] === undefined) {
                    result[key] = input[key];
                }
            }
        }

        // Second pass to remove already expanded composite keys.
        for (const key of keysToRemove) {
            delete result[key];
        }

        // Third pass to apply nest recursively
        for (const key of Array.from(keysToNest)) {
            result[key] = qb.nest(result[key] as Record<string, unknown>);
        }

        return result;
    }

}
