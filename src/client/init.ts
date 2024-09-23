/* eslint-disable @typescript-eslint/no-explicit-any */
import { type PoolConfig, type QueryConfig, type QueryResult, type Pool, type QueryConfigValues } from 'pg'

import pg from 'pg'

// DO NOT IMPORT IT LIKE { Pool: PoolC, types } = pg 
// It causes a bug that fails to initialize the DB with an error on build of the main project (but works ok on dev).
// The error is "PoolC is not a constructor". 
// It is probably caused by the "pg" library being a CommonJS module and rollup/vite failing to properly convert it to ESM.
// As a result it skips bundling the library which results in "PoolC" being undefined.
// In development Vite uses `esbuild` instead of `rollup`, this one is able to do the conversion properly.
// Both `rollup` and `esbuild` can do the conversion properly if you use this syntax instead:
const PoolC = pg.Pool
const types = pg.types

// And if you wondered, yes, vite/rollup will actually check this source code to determine what dependencies to include,
// all IN THE NAME OF PERFORMANCE.

// I forgot the reason why we're doing this instead of using pg.Pool everywhere, but there was a reason! Probably...
// I wouldn't risk it.

import configDefault, { Config } from '../config.js'

/* THIS IS A FIX FOR node-postgres,
// it is simply using a newer version of the parse function extracted from 
// postgres-types node-pg-types in pg (2.x), node-pg-types in master branch (4.x)
*/
import parseDate from 'postgres-date'

const parseTimestamp = function (value): Date | number | null {
    const utc = value.endsWith(' BC')
        ? value.slice(0, -3) + 'Z BC'
        : value + 'Z'

    return parseDate(utc)
}

types?.setTypeParser(types.builtins.TIMESTAMP, parseTimestamp)

/* -------------------------------------------------------------------------- */

export const clientSQLConfig = {
    printSQLQueries: false,
    printSQLValues: false
}

export let client: Pool = null
export const pools: Map<string, Pool> = new Map()

class ClientWrapper extends PoolC {
    query<R extends any[] = any[], I = any[]>(textOrConfig: string | QueryConfig<I>, values?: QueryConfigValues<I>): Promise<QueryResult<R>> {
        if (clientSQLConfig.printSQLQueries) console.log("SQL:", textOrConfig)
        if (clientSQLConfig.printSQLValues) console.log("SQL VALUES:", values)
        return super.query(textOrConfig, values);
    }
}

export function getClient(): Pool {
    if (client == null) throw new Error("Client has not been initialized!")
    return client
}

/**
 * Initializes the SQL client to be used during a normal run on the application.
 * 
 * This function creates a pool (see `pools`) for every role defined in config 
 * except SA.
 * 
 * The default pool used by the exported client object is the one for the role
 * "api" unless overridden with the "user" option. All the pools configuration 
 * can also be overridden using `options`.
 * 
 * 
 * @param config 
 * @param options 
 */
export function clientInit(configPartial: Partial<Config> = {}, options: PoolConfig = {}): void {
    const config = { ...configDefault, ...configPartial }
    configDefault.db = config.db
    for (const [key, role] of Object.entries(config.db.roles)) {
        pools.set(
            key,
            new ClientWrapper({ ...config.db.connection, ...options, ...role } as PoolConfig)
        )
    }
    client = pools.get(options.user ?? "api")
}