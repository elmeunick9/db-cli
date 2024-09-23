import readline from 'readline'
import config from '../config'
import { DB_ERROR } from "../errors"
import tools from "../tools"
import { MigrationGraph, MigrationGraphNode } from "../tools/fs"
import pg from 'pg'
import * as info from './info'
import * as schemaManager from './schema'
import { executeSqlFile } from '../client'
import { prefixSchema, unPrefixSchema } from '../tools/string_utils'
import { generateDBSchema } from '../generator'
import { DBSchema } from '../parser/enhance'
import { qb } from '../generator/lib/queryBuilder'

/**
 * Use internally to prompt the user. This is recommended for helping debug
 * migrations.
 * 
 * @param query 
 * @returns 
 */
function askQuestion(query: string): Promise<string> {
    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout,
    });

    return new Promise(resolve => rl.question(query, ans => {
        rl.close();
        resolve(ans);
    }))
}

/**
 * Creates a list of all tables in given schema.
 */
async function listAllTables(schema = "public"): Promise<string[]> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    await client.connect()
    const response = await client.query(`SELECT * FROM pg_tables WHERE schemaname='${prefixSchema(schema)}';`)
    const result = response.rows.map(x => x.tablename)
    client.end()
    return result
}

interface EnumReference {
    schema: string
    name: string
}

/**
 * Creates a list of all enums and in which schema they appear.
 */
async function listAllEnums(): Promise<EnumReference[]> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    await client.connect()
    const response = await client.query(`
SELECT
    n.nspname AS "schema",  
    t.typname AS "name"
FROM pg_type t 
    JOIN pg_enum e ON t.oid = e.enumtypid  
    JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace
WHERE
	t.oid = e.enumtypid
GROUP BY (n.nspname, t.typname)`
    )
    const result = response.rows
    client.end()
    return result
}

/**
 * Counts the amount of rows on a given table. Can count at most 5K rows.
 */
async function countTableRows(schema: string, table: string, limit = 5000): Promise<number> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    await client.connect()
    const response = await client.query(`
        SELECT COUNT(*) FROM ( 
            SELECT * FROM "${schema}"."${table}" LIMIT ${limit}
        ) AS x;
    `)
    const result = parseInt(response.rows[0]?.count)
    await client.end()
    return result
}

interface TableCount {
    schema: string
    table: string
    count: number
}

async function generateSchemaTableCount(schemas: string[]): Promise<TableCount[]> {
    const tableCounts = []
    for (const schema of schemas) {
        const tables = await listAllTables(schema)
        for (const table of tables) {
            const count = await countTableRows(schema, table)
            tableCounts.push({ schema, table, count })
        }
    }
    return tableCounts
}

async function createEnumTypeCasts(fromSchemas: string[], toSchemas: string[]): Promise<void> {

    async function createCast(from: EnumReference, to: EnumReference): Promise<void> {
        const sql = 
            `CREATE CAST ("${from.schema}"."${from.name}" AS "${to.schema}"."${to.name}") WITH INOUT AS IMPLICIT;`
        const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})
    
        console.log("Executing SQL:", sql)
        await client.connect()
        await client.query(sql)
        await client.end()
    }

    const enums = await listAllEnums()
    for (const schema of fromSchemas.filter(x => toSchemas.includes(x))) {
        const fromEnums = enums.filter(x => x.schema == `o_${schema}`)
        const toEnums = enums.filter(x => x.schema == schema)

        for (const { name } of fromEnums) {
            const from = fromEnums.find(x => x.name == name)
            const to = toEnums.find(x => x.name == name)
            if (!from || !to) continue
            await createCast(from, to)
        }
    }
}

async function autoInsertSelect(schema: DBSchema, schemaName: string, tableName: string): Promise<void> {
    const dest = `"${prefixSchema(schemaName)}"."${tableName}"`
    const from = `"o_${prefixSchema(schemaName)}"."${tableName}"`
    const columns = qb.columnList(schema[unPrefixSchema(schemaName)].tables[tableName].columns.map(x => x.name))
    const sql = `INSERT INTO ${dest} (${columns}) SELECT ${columns} FROM ${from};`

    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    console.log("Executing SQL:", sql)
    await client.connect()
    await client.query(sql)
    await client.end()
}


// =============================================================================

/**
 * Returns a path (list) of what migrations need to be applied in order to go
 * from one version to another.
 * 
 * Any given version can have multiple migrations defined, both forward and
 * backwards. This crates a migration graph. This algorithm will return the
 * shortest path, i.e. the path going from A to B where the latest amount of
 * migrations are needed.
 * 
 * @param from Origin version.
 * @param to Final version.
 * @returns List of versions.
 */
export function createMigrationPlan(from: string, to = "api"): string[] {
    from = tools.findVersion(from)
    to = tools.findVersion(to)
    if (from === to) return []

    /**
     * Fills the "path" property on each node of the graph with the shortest
     * path to the origin.
     */
    const calculateDistances = (graph: MigrationGraph, origin: string): MigrationGraph => {
        const getNode = (version: string): MigrationGraphNode => graph.find(x => x.version === version)
        const dfs = (version: string, path = []): void => {
            const currentNode = getNode(version)
            if (!currentNode.path || currentNode.path.length > path.length) {
                currentNode.path = path
            }
            for (const target of currentNode.targets) {
                const node = getNode(target)
                if (!node.path || node.path.length > path.length + 1) {
                    dfs(target, [...path, version])
                }
            }
        }
        dfs(origin)
        return graph
    }

    const graph = calculateDistances( tools.createMigrationGraph(), to )
    const path = graph.find(x => x.version == from).path

    if (!path) throw DB_ERROR.MIGRATION_PATH_NOT_FOUND

    return path.reverse() ?? []
}

/**
 * Perform a single migration step from one version to another. The migration file
 * with the format YYYYMMDD.sql must exists. The version in the file name is the
 * source version supported, (e.g sql/next/20210101.sql) would be executed for
 * migrating from version 20210101 to version "next".
 * 
 * If in development mode, we pause at key steps to allow for debugging.
 */
export async function migrateStep(from: string, to: string): Promise<void> {
    console.log("Migration:", from, "-->", to)

    const state : info.MetaState = await info.getMetaByKey('state') as info.MetaState
    const fromSchemas = tools.findSQLSchemas(from, true)
    const toSchemas = tools.findSQLSchemas(to, true)
    if (state?.migration !== 'ready') throw DB_ERROR.INVALID_STATE

    if (config.isDevelopment()) {
        await askQuestion("Migration ready, paused before start. Press [ENTER] to continue.");
    }

    await info.setMetaByKey('state', {...state, migration: 'in-progress'})
    await schemaManager.renameSchemas( fromSchemas.map(x => [x, `o_${x}`]) )
    try {
        for (const schema of toSchemas) {
            await schemaManager.createSchema(schema, to)
        }
        await schemaManager.initializePublicSchema(to, 'in-progress')
        await createEnumTypeCasts(fromSchemas, toSchemas)
    
        if (config.isDevelopment()) {
            await askQuestion("Migration paused before executing script. Press [ENTER] to continue.");
        }
    
        if (!config.db.prefix) {
            await executeSqlFile(
                `sql/${to}/${from}.sql`,
                {...config.db.connection, ...config.db.roles.sa},
                [`SET session_replication_role = replica;`],
                [`SET session_replication_role = DEFAULT;`])
        } else {
            await executeSqlFile(
                `sql/${to}/${from}.sql`,
                {...config.db.connection, ...config.db.roles.sa})
        }
    
        const tablesInOrigin = await generateSchemaTableCount(fromSchemas.map(x => `o_${x}`))
        const tablesToMove = (await generateSchemaTableCount(toSchemas))
            .filter( x => x.count == 0)
            .filter (x => tablesInOrigin.find(y => y.schema === `o_${x.schema}` && y.table === x.table && y.count > 0))
            .map(x => [x.schema, x.table])
    
        for (const [schema, table] of tablesToMove) {
            const dbSchema = generateDBSchema(from)
            await autoInsertSelect(dbSchema, schema, table)
        }
    } catch (e) {
        console.log("ERROR! ROLLING BACK TO VERSION", from)
        console.log(e)
        await schemaManager.deleteSchemas( toSchemas.reverse() )
        await schemaManager.renameSchemas( fromSchemas.map(x => [`o_${x}`, x]) )
        await info.setMetaByKey('state', {...state, migration: 'ready'})
        throw e
    }

    await schemaManager.deleteSchemas(fromSchemas.map(x => `o_${x}`).reverse())
    await info.setMetaByKey('state', {...state, migration: 'ready'})
}


