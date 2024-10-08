import readline from 'node:readline'
import config from '../config'
import { DB_ERROR } from "../errors"
import tools from "../tools"
import { loadSQLFile, MigrationGraph, MigrationGraphNode, parseCommand } from "../tools/fs"
import * as info from './info'
import * as schemaManager from './schema'
import { CommandExecutionSet, executeSqlFile } from '../client'
import { prefixSchema, unPrefixSchema, unquote } from '../tools/string_utils'
import { generateDBSchema } from '../generator'
import { DBSchema } from '../parser/enhance'
import { qb } from '../generator/lib/queryBuilder'
import { IClient, MigrationOptions } from '../interfaces'

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
async function listAllTables(client: IClient, schema = "public"): Promise<string[]> {
    const response = await client.query(`SELECT * FROM pg_tables WHERE schemaname='${prefixSchema(schema)}';`)
    const result = response[0].rows.map(x => x.tablename)
    return result
}

interface EnumReference {
    schema: string
    name: string
}

/**
 * Creates a list of all enums and in which schema they appear.
 */
async function listAllEnums(client: IClient): Promise<EnumReference[]> {
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
    const result = response[0].rows
    return result
}

/**
 * Counts the amount of rows on a given table. Can count at most 5K rows.
 */
async function countTableRows(client: IClient, schema: string, table: string, limit = 5000): Promise<number> {
    const response = await client.query(`
        SELECT COUNT(*) FROM ( 
            SELECT * FROM "${schema}"."${table}" LIMIT ${limit}
        ) AS x;
    `)
    const result = parseInt(response[0]?.rows[0]?.count)
    return result
}

interface TableCount {
    schema: string
    table: string
    count: number
}

async function generateSchemaTableCount(client: IClient, schemas: string[]): Promise<TableCount[]> {
    const tableCounts = []
    for (const schema of schemas) {
        const tables = await listAllTables(client, schema)
        for (const table of tables) {
            const count = await countTableRows(client, schema, table)
            tableCounts.push({ schema, table, count })
        }
    }
    return tableCounts
}

async function createEnumTypeCasts(client: IClient, fromSchemas: string[], toSchemas: string[]): Promise<void> {

    async function createCast(from: EnumReference, to: EnumReference): Promise<void> {
        const sql = 
            `CREATE CAST ("${from.schema}"."${from.name}" AS "${to.schema}"."${to.name}") WITH INOUT AS IMPLICIT;`
    
        await client.query(sql)
    }

    const enums = await listAllEnums(client)
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

async function autoInsertSelect(client: IClient, schema: DBSchema, schemaName: string, tableName: string): Promise<void> {
    const dest = `"${prefixSchema(schemaName)}"."${tableName}"`
    const from = `"o_${prefixSchema(schemaName)}"."${tableName}"`
    const columns = qb.columnList(schema[unPrefixSchema(schemaName)].tables[tableName].columns.map(x => x.name))
    const sql = `INSERT INTO ${dest} (${columns}) SELECT ${columns} FROM ${from};`

    await client.query(sql)
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
export async function migrateStep(client: IClient, opts: MigrationOptions, from: string, to: string, prefix: string): Promise<void> {
    opts.logger.info("Migration:", from, "-->", to)

    const state : info.MetaState = await info.getMetaByKey(client, opts, 'state') as info.MetaState
    const commands = loadSQLFile(`sql/${to}/${from}.sql`, { mode: 'commands' }).map(parseCommand)
    const fromSchemas = tools.findSQLSchemas(from, true, prefix)
        .filter(x => !commands.find(c => c.name === "skip-schema" && c.args.map(unquote).includes(unPrefixSchema(x, prefix))))
    const toSchemas = tools.findSQLSchemas(to, true, prefix)
        .filter(x => !commands.find(c => c.name === "skip-schema" && c.args.map(unquote).includes(unPrefixSchema(x, prefix))))
    if (state?.migration !== 'ready') throw DB_ERROR.INVALID_STATE

    if (config.isDevelopment()) {
        await askQuestion("Migration ready, paused before start. Press [ENTER] to continue.");
    }

    await info.setMetaByKey(client, opts, 'state', {...state, migration: 'in-progress'})
    if (fromSchemas.length > 0) {
        await schemaManager.renameSchemas(client, opts.db, fromSchemas.map(x => [x, `o_${x}`]) )
    }
    try {
        for (const schema of toSchemas) {
            await schemaManager.createSchema(client, opts, schema, to)
        }

        if (toSchemas.length > 0) {
            await client.connect({ ...opts.db.roles.sa })
            await schemaManager.initializePublicSchema(client, opts, to, 'in-progress')
            await createEnumTypeCasts(client, fromSchemas, toSchemas)
        }
    
        if (config.isDevelopment()) {
            await askQuestion("Migration paused before executing script. Press [ENTER] to continue.");
        }

        const commandSet: CommandExecutionSet = {
            "include": async (client: IClient, args: string[]) => {
                const file = unquote(args[0])
                const split = !file.endsWith('function.sql')
                await executeSqlFile(client, `sql/${to}/${file}`, [], [], split)
            }
        }
    
        if (!prefix && toSchemas.length > 0) {
            await executeSqlFile(
                client,
                `sql/${to}/${from}.sql`,
                [`SET session_replication_role = replica;`],
                [`SET session_replication_role = DEFAULT;`],
                true, commandSet)
        } else {
            await executeSqlFile(
                client,
                `sql/${to}/${from}.sql`, 
                [], [], true, commandSet)
        }
        client.end()
    
        if (fromSchemas.length > 0 && toSchemas.length > 0) {
            const tablesInOrigin = await generateSchemaTableCount(client, fromSchemas.map(x => `o_${x}`))
            const tablesToMove = (await generateSchemaTableCount(client, toSchemas))
                .filter( x => x.count == 0)
                .filter (x => tablesInOrigin.find(y => y.schema === `o_${x.schema}` && y.table === x.table && y.count > 0))
                .map(x => [x.schema, x.table])
        
            for (const [schema, table] of tablesToMove) {
                const dbSchema = generateDBSchema(from)
                await autoInsertSelect(client, dbSchema, schema, table)
            }
        }
    } catch (e) {
        opts.logger.error("ERROR! ROLLING BACK TO VERSION", from)
        opts.logger.info(e)
        await schemaManager.deleteSchemas(client, opts.db, toSchemas.reverse() )
        await schemaManager.renameSchemas(client, opts.db, fromSchemas.map(x => [`o_${x}`, x]) )
        await info.setMetaByKey(client, opts, 'state', {...state, migration: 'ready'})
        throw e
    }

    await schemaManager.deleteSchemas(client, opts.db, fromSchemas.map(x => `o_${x}`).reverse())
    await info.setMetaByKey(client, opts,'state', {...state, migration: 'ready'})
}


