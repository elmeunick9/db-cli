import * as clientLib from "../../client"
import { DB_ERROR } from "../../errors"
import { qb, format, BaseType, Clause, Expand, ExpandedColumnAlias, ExpandedExpression  } from "./queryBuilder"
export { type Clause, type BaseType } from "./queryBuilder";
import oasLib from './oas'
import * as stringUtils from '../../tools/string_utils'
import { DBSchema } from "../../parser/enhance"
import { IClient } from "../../interfaces";

export const sql = clientLib.sql
export const oas = oasLib
export const string_utils = stringUtils

export interface ColumnOrderFilter<T> {
    column: T
    order: "ASC" | "DESC"
    nullsFirst?: boolean
}

export interface Options {
    verbose?: boolean
}

export interface ListOptions<T extends string> extends Options {
    limit?: number
    offset?: number
    orderBy?: ColumnOrderFilter<T>[]
    filter?: Clause<T>
    columns?: T[]
    expand?: Expand<T>[]
}

export interface ReadOptions<T extends string> extends Options {
    expand?: Expand<T>[]   // FK(this -> other) ONE-TO-ONE
//    extend?: never      // FK(other -> this) ONE-TO-MANY
}

export type kObject = {[key: string]: BaseType|undefined}

export const generic = {
    list: async <K extends string, T>(client: IClient, schema: string, table: string, optionsP: Partial<ListOptions<K>> = {}, values: kObject = {}, db_schema: DBSchema|undefined): Promise<T[]> => {
        const options = { columns: [], orderBy: [], ...optionsP }
        const join = (ex: Expand<K>, alias: string): ExpandedExpression => qb.expand(db_schema, schema, table, ex, alias, "")
        const alias = (c: ExpandedColumnAlias): string => `${c.column} AS "${c.alias}"`
        const tn = (n = 4): string => ",\n" + "    ".repeat(n)
        const expanded = options.expand?.map((x, i) => join(x, `_join${i}`)) ?? []
        const where = options.filter != null ? qb.whereClause(options.filter) : ""
        const orderBy = `${options.orderBy.map(x => `"${x.column}" ${x.order}${x.nullsFirst ? " NULLS FIRST" : ""}`).join(', ')}`
        const filter = sql
           `${options.filter != null ? `WHERE ${where}` : ''}
            ${options.orderBy.length > 0 ? `ORDER BY ${orderBy}` : ''}
            ${options.limit != null ? `LIMIT ${options.limit}` : ''}
            ${options.offset != null ? `OFFSET ${options.offset}` : ''}`
        const query = expanded.length == 0
            ? format(
                sql`
                    SELECT ${options.columns.length > 0 ? qb.columnList(options.columns) : '*'}  
                    FROM "${client.settings().prefix}${schema}"."${table}"
                    ${filter}
                `, 
                values as kObject
            )
            : format(
                sql`
                    SELECT
                    ${options.columns.length > 0 ? qb.columnList(options.columns, `"${table}".`) : `"${table}".*`},
                        ${expanded.map(x => x.columns.map(alias).join(tn(6))).join(tn(6))}
                    FROM "${client.settings().prefix}${schema}"."${table}"
                    ${expanded.map(x => x.text).join("\n")}
                    ${filter}
                `, 
                values as kObject
            )

        const response = await client.query(query)
        if (expanded.length == 0) return response[0].rows
        else return response[0].rows.map(row => qb.nest(row) as T)
    },
    createUsingDefaultKey: async <K, V>(client: IClient, schema: string, table: string, keyShape: K, value: V): Promise<K> => {
        const query = format(
            sql`
                INSERT INTO "${client.settings().prefix}${schema}"."${table}" (${qb.columnList(value as {[key: string]: unknown})})
                VALUES (${qb.valueList(value as {[key: string]: unknown})})
                RETURNING (${qb.columnList(keyShape as {[key: string]: unknown})})
            `, 
            value as kObject
        )

        const response = await client.query(query)
        if (response[0].rows.length === 0) throw new Error(DB_ERROR.CREATE_FAILED)
        if (response[0].rows.length > 1) throw new Error(DB_ERROR.CORRUPTED)
        return response[0].rows[0]
    },
    readByKey: async <K, V, T extends string = string>(client: IClient, schema: string, table: string, key: K, options: ReadOptions<T> = {}, db_schema?: DBSchema ): Promise<V> => {
        const join = (ex: Expand<T>, i: number): ExpandedExpression => qb.expand(db_schema, schema, table, ex, `_join${i}`, "")
        const alias = (c: ExpandedColumnAlias): string => `${c.column} AS "${c.alias}"`
        const expanded = options.expand?.map((x, i) => join(x, i)) ?? []
        const tn = (n = 4): string => ",\n" + "    ".repeat(n)
        const query = expanded.length == 0
            ? format(
                sql`
                    SELECT * FROM "${client.settings().prefix}${schema}"."${table}" 
                    WHERE ${qb.whereMatchList(key as {[key: string]: unknown})}
                `, 
                key as kObject
            )
            : format(
                sql`
                    SELECT
                        "${table}".*,
                        ${expanded.map(x => x.columns.map(alias).join(tn(6))).join(tn(6))}
                    FROM "${client.settings().prefix}${schema}"."${table}"
                    ${expanded.map(x => x.text).join("\n")}
                    WHERE ${qb.whereMatchList(key as {[key: string]: unknown}, `"${table}".`)}
                `, 
                key as kObject
            )

        const response = await client.query(query)
        if (response[0].rows.length === 0) throw new Error(DB_ERROR.NOT_FOUND)
        if (response[0].rows.length > 1) throw new Error(DB_ERROR.CORRUPTED)
        
        const row = response[0].rows[0]
        if (expanded.length == 0) return row
        return qb.nest(row) as V
    },
    updateByKey: async <K, V>(client: IClient, schema: string, table: string, key: K, value: V): Promise<void> => {
        const query = format(
            sql`
                UPDATE "${client.settings().prefix}${schema}"."${table}"
                SET ${qb.assignList(value as {[key: string]: unknown})}
                WHERE (${qb.whereMatchList(key as {[key: string]: unknown})})
            `, 
            {...key, ...value} as {[key: string]: BaseType}
        )

        const response = await client.query(query)
        if (response[0].rowCount == 0) throw new Error(DB_ERROR.UPDATE_FAILED)
    },
    deleteByKey: async <K>(client: IClient, schema: string, table: string, key: K): Promise<void> => {
        const query = format(
            sql`
                DELETE FROM "${client.settings().prefix}${schema}"."${table}" 
                WHERE ${qb.whereMatchList(key as {[key: string]: unknown})}
            `, 
            key as kObject
        )

        const response = await client.query(query)
        if (response[0].rowCount == 0) throw new Error(DB_ERROR.DELETE_FAILED)
    },
    deleteByFilter: async <T>(client: IClient, schema: string, table: string, filter: Clause<T>, values: kObject = {}): Promise<void> => {
         const query = format(
            sql`
                DELETE FROM "${client.settings().prefix}${schema}"."${table}"
                WHERE ${qb.whereClause(filter)}
            `, 
            values as kObject
        )

        await client.query(query)
    },
    push: async <V>(client: IClient, schema: string, table: string, values: V): Promise<void> => {
        const query = format(
            sql`
                INSERT INTO "${client.settings().prefix}${schema}"."${table}" (${qb.columnList(values as {[key: string]: unknown})})
                VALUES (${qb.valueList(values as {[key: string]: unknown})})
            `, 
            values as kObject
        )

        const response = await client.query(query)
        if (response[0].rowCount == 0) throw new Error(DB_ERROR.CREATE_FAILED)
    },
    pop: async <K, V>(client: IClient, schema: string, table: string, key: K): Promise<V> => {
        const query = format(
            sql`
                DELETE FROM "${client.settings().prefix}${schema}"."${table}" 
                WHERE ${qb.whereMatchList(key as {[key: string]: unknown})}
                RETURNING *
            `, 
            key as kObject
        )

        const response = await client.query(query)
        if (response[0].rowCount == 0) throw new Error(DB_ERROR.DELETE_FAILED)
        if (response[0].rows.length == 0) throw new Error(DB_ERROR.CORRUPTED)
        if (response[0].rows.length >  1) throw new Error(DB_ERROR.CORRUPTED)
        return response[0].rows[0]
    },
    incrementByKey: async <K, V>(client: IClient, schema: string, table: string, key: K, value: V): Promise<void> => {
        const query = format(
            sql`
                UPDATE "${client.settings().prefix}${schema}"."${table}"
                SET ${qb.incrementList(value as {[key: string]: unknown})}
                WHERE (${qb.whereMatchList(key as {[key: string]: unknown})})
            `, 
            {...key, ...value} as {[key: string]: BaseType}
        )

        const response = await client.query(query)
        if (response[0].rowCount == 0) throw new Error(DB_ERROR.UPDATE_FAILED)
    },
}

export default { sql, generic, qb, format, oas, string_utils }
export type { IClient, DBSchema } 