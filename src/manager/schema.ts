import pg from 'pg'
import config from "../config.js"
import { findSQLFiles, findVersion } from '../tools/core.js'
import { executeSqlFile } from '../client/index.js'
import { MetaInfo } from './info.js'
import { prefixSchema } from '../tools/string_utils.js'

export async function createSchema(name = 'public', version = 'next'): Promise<void> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    const roles = config.db.roles
    const user = roles.api.user

    await client.connect()
    if (name === "public") {
        await client.query(`CREATE SCHEMA IF NOT EXISTS ${prefixSchema(name)};`)
        //await client.query(`ALTER SCHEMA "${prefixSchema(name)}" OWNER TO ${user}`);
    }
    else await client.query(`CREATE SCHEMA ${prefixSchema(name)};`)

    for (const filepath of findSQLFiles({ version, schema: name })) {
        await executeSqlFile(filepath, client, [`SET SCHEMA '${prefixSchema(name)}';`])
    }

    for (const filepath of findSQLFiles({ version, schema: name, type: 'function', allowMultiple: true })) {
        await executeSqlFile(filepath, client, [`SET SCHEMA '${prefixSchema(name)}';`], [], false)
    }

    for (const filepath of findSQLFiles({ version, schema: name, type: 'insert' })) {
        await executeSqlFile(filepath, client, [`SET SCHEMA '${prefixSchema(name)}';`])
    }

    await client.query(`GRANT SELECT, UPDATE, INSERT, DELETE ON ALL TABLES IN SCHEMA ${prefixSchema(name)} TO ${user};`)
    await client.query(`GRANT USAGE ON ALL SEQUENCES IN SCHEMA ${prefixSchema(name)} TO ${user};`)
    await client.query(`GRANT USAGE ON SCHEMA ${prefixSchema(name)} TO ${user};`)
    await client.end()
}

export async function initializePublicSchema(version: string, migration_state = 'ready'): Promise<void> {
    const info : MetaInfo = {
        'version': findVersion(version),
        'description': config.app.description ?? "",
        'creation-date': (new Date()).toISOString(),
        'source': config.app.source ?? ""
    }
    const state = { migration: migration_state }
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    await client.connect()
    await client.query(
        `CREATE TABLE "${prefixSchema("public")}"."meta" (
            "key"               varchar(8)          ,
            "value"             json                NOT NULL,
            PRIMARY KEY ("key")
        );`
    )
    await client.query(`INSERT INTO "${prefixSchema("public")}"."meta" VALUES ('info' , '${JSON.stringify(info)}');`)
    await client.query(`INSERT INTO "${prefixSchema("public")}"."meta" VALUES ('state' , '${JSON.stringify(state)}');`)
    await client.end()
}

export async function initializeSchema(schema: string, version: string): Promise<void> {
    if (schema === 'public' || schema === prefixSchema("public")) {
        await initializePublicSchema(version)
    }

    if (findSQLFiles({ version, schema, type: 'snapshot' }).length > 0) {
        //await sys.applySnapshoot(schema, version) 
    } else {
        for (const filepath of findSQLFiles({ version, schema, type: 'default' })) {
            await executeSqlFile(filepath, {...config.db.connection, ...config.db.roles.sa}, [`SET SCHEMA '${prefixSchema(schema)}';`])
        }
    }
}

/**
 * Rename all schemas for the name in the first element of the pair to the name
 * in the second element of the pair.
 * 
 * @param schemasMapping A list of pairs of names.
 */
export async function renameSchemas(schemasMapping: [old: string, new: string][]): Promise<void> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    await client.connect()
    for (const [oldName, newName] of schemasMapping) {
        await client.query(`ALTER SCHEMA "${prefixSchema(oldName)}" RENAME TO "${prefixSchema(newName)}"`)
    }
    await client.end()
}

/**
 * Delete all schemas listed if they exists.
 * 
 * @param schemas A list of schema names.
 */
export async function deleteSchemas(schemas: string[]): Promise<void> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    await client.connect()
    for (const schema of schemas) {
        const sql = `DROP SCHEMA IF EXISTS "${prefixSchema(schema)}" CASCADE`
        console.log("Executing SQL:", sql)
        await client.query(sql)
    }
    await client.end()
}

