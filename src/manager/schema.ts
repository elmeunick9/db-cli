import { findSQLFiles, findVersion } from '../tools/core.js'
import { executeSqlFile } from '../client/index.js'
import { MetaInfo } from './info.js'
import { prefixSchema } from '../tools/string_utils.js'
import { IClient, InitOptions } from "../interfaces.js"
import { Config } from '../config.js'

export async function createSchema(client: IClient, opts: InitOptions, name = 'public', version = 'next'): Promise<void> {
    await client.connect({...opts.db.roles.sa})

    const roles = opts.db.roles
    const user = roles.api.user

    if (name === opts.db.publicSchema) {
        await client.query(`CREATE SCHEMA IF NOT EXISTS ${prefixSchema(name, opts.db.prefix)};`)
        //await client.query(`ALTER SCHEMA "${prefixSchema(name)}" OWNER TO ${user}`);
    }
    else await client.query(`CREATE SCHEMA ${prefixSchema(name, opts.db.prefix)};`)

    for (const filepath of findSQLFiles({ version, schema: name })) {
        await executeSqlFile(client, filepath, [`SET SCHEMA '${prefixSchema(name, opts.db.prefix)}';`])
    }

    for (const filepath of findSQLFiles({ version, schema: name, type: 'function', allowMultiple: true })) {
        await executeSqlFile(client, filepath, [`SET SCHEMA '${prefixSchema(name, opts.db.prefix)}';`], [], false)
    }

    for (const filepath of findSQLFiles({ version, schema: name, type: 'insert' })) {
        await executeSqlFile(client, filepath, [`SET SCHEMA '${prefixSchema(name, opts.db.prefix)}';`])
    }

    await client.query(`GRANT SELECT, UPDATE, INSERT, DELETE ON ALL TABLES IN SCHEMA ${prefixSchema(name, opts.db.prefix)} TO ${user};`)
    await client.query(`GRANT USAGE ON ALL SEQUENCES IN SCHEMA ${prefixSchema(name, opts.db.prefix)} TO ${user};`)
    await client.query(`GRANT USAGE ON SCHEMA ${prefixSchema(name, opts.db.prefix)} TO ${user};`)
    await client.end()
}

export async function initializePublicSchema(client: IClient, opts: InitOptions, version: string, migration_state = 'ready'): Promise<void> {
    const info : MetaInfo = {
        'version': findVersion(version),
        'description': opts.meta.description ?? "",
        'creation-date': (new Date()).toISOString(),
        'source': opts.meta.source ?? ""
    }
    const state = { migration: migration_state }

    await client.query(
        `CREATE TABLE "${prefixSchema(opts.db.publicSchema, opts.db.prefix)}"."meta" (
            "key"               varchar(8)          ,
            "value"             json                NOT NULL,
            PRIMARY KEY ("key")
        );`
    )
    await client.query(`INSERT INTO "${prefixSchema(opts.db.publicSchema, opts.db.prefix)}"."meta" VALUES ('info' , '${JSON.stringify(info)}');`)
    await client.query(`INSERT INTO "${prefixSchema(opts.db.publicSchema, opts.db.prefix)}"."meta" VALUES ('state' , '${JSON.stringify(state)}');`)
}

export async function initializeSchema(client: IClient, opts: InitOptions, schema: string, version: string): Promise<void> {
    client.connect({...opts.db.roles.sa})
    if (schema === opts.db.publicSchema || schema === prefixSchema(opts.db.publicSchema, opts.db.prefix)) {
        await initializePublicSchema(client, opts, version)
    }

    if (findSQLFiles({ version, schema, type: 'snapshot' }).length > 0) {
        //await sys.applySnapshoot(schema, version) 
    } else {
        for (const filepath of findSQLFiles({ version, schema, type: 'default' })) {
            await executeSqlFile(client, filepath, [`SET SCHEMA '${prefixSchema(schema, opts.db.prefix)}';`])
        }
    }
    client.end()
}

/**
 * Rename all schemas for the name in the first element of the pair to the name
 * in the second element of the pair.
 * 
 * @param schemasMapping A list of pairs of names.
 */
export async function renameSchemas(client: IClient, db: Config['db'], schemasMapping: [old: string, new: string][]): Promise<void> {
    await client.connect({...db.roles.sa})

    for (const [oldName, newName] of schemasMapping) {
        await client.query(`ALTER SCHEMA "${prefixSchema(oldName, db.prefix)}" RENAME TO "${prefixSchema(newName, db.prefix)}"`)
    }

    await client.end()
}

/**
 * Delete all schemas listed if they exists.
 * 
 * @param schemas A list of schema names.
 */
export async function deleteSchemas(client: IClient, db: Config['db'], schemas: string[]): Promise<void> {
    await client.connect({...db.roles.sa})

    for (const schema of schemas) {
        const sql = `DROP SCHEMA IF EXISTS "${prefixSchema(schema, db.prefix)}" CASCADE`
        await client.query(sql)
    }

    await client.end()
}

