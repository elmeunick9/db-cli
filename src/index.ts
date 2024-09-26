import config from './config'
import manager from './manager'
import tools from './tools/index.js'
import { DB_ERROR } from './errors.js'
import { MetaInfo } from './manager/info.js'
import fs from 'fs'
import { generateDBSchema, writeJSON, WriteJsonOptions, writeLibrary, WriteLibraryOptions } from './generator'
import * as stringUtils from './tools/string_utils';
import { DatabaseError, IClient, InitOptions, MigrationOptions } from './interfaces'
import * as lib from './generator/lib'
export * as lib from './generator/lib'

/**
 * Initializes the DB when on development mode. It will delete and recreate the DB
 * to the version specified. 
 * 
 * @param from 
 */
export async function init(client: IClient, options: Partial<InitOptions> = {}): Promise<void> {
    const opts: InitOptions = {
        development: config.isDevelopment(),
        from: config.isDevelopment() ? 'next' : 'latest',
        clean: config.isDevelopment(),
        createUsers: true,
        logger: console,
        terminate: config.isDevelopment(),
        db: config.db,
        meta: {
            description: '',
            source: '',
        },
        ...{
            ...options,
            db: { ...config.db, ...(options.db ?? {}) }
        }
    }
    try {
        if (opts.clean) {
            opts.logger.log("Cleaning DB.")
            await manager.createCleanDB(client, opts)
        }
        if (opts.createUsers) {
            await manager.createUsers(client, opts)
        }
        for (const schema of tools.findSQLSchemas(opts.from, true)) {
            await manager.createSchema(client, opts, schema, opts.from)
        }
        for (const schema of tools.findSQLSchemas(opts.from, true)) {
            await manager.initializeSchema(client, opts, schema, opts.from)
        }
        
        const info = await manager.getMetaByKey(client, opts, 'info') as MetaInfo
        if (!info.version) throw DB_ERROR.CORRUPTED

        opts.logger.log("DB initialized.")
    } catch (e) {
        if ((e as DatabaseError).code === '42P04') {
            opts.logger.error("DB already exists! Aborted.")
        } else {
            opts.logger.error(e)
            opts.logger.info(JSON.stringify(e))
        }
    }
}

/**
 * Migrates the DB to the version specified. 
 * @param from Target version or alias.
 */
export async function migrate(client: IClient, options: Partial<MigrationOptions> = {}, to = config.isDevelopment() ? 'next' : 'latest'): Promise<void> {
    const opts: MigrationOptions = {
        development: config.isDevelopment(),
        from: config.isDevelopment() ? 'next' : 'latest',
        clean: config.isDevelopment(),
        createUsers: true,
        logger: console,
        terminate: config.isDevelopment(),
        db: config.db,
        meta: {
            description: '',
            source: '',
        },
        to: config.isDevelopment() ? 'next' : 'latest',
        ...options,
    }

    try {
        const info = await manager.getMetaByKey(client, opts, 'info') as MetaInfo
        if (!info.version) throw DB_ERROR.CORRUPTED
    
        const plan = manager.createMigrationPlan(info.version, to)
        let current = info.version
        if (plan.length === 0) {
            opts.logger.log("Already on last release, skipping migration.")
        }
        for (const step of plan) {
            await manager.migrateStep(client, opts, current, step)
            current = step
        }
    } catch (e) {
        opts.logger.error(e)
    }
}

/**
 * Creates a new release.
 * 
 * This involves renaming "next" -> YYYYMMDD and creating a new "next" folder.
 * 
 * @param from Target version or alias.
 */
export async function release(version: string): Promise<void> {
    try {
        const dest = `sql/${version}`
        fs.mkdirSync(dest, { recursive: true });
        tools.copyDirectoryRecursive("sql/next", dest)
        tools.removeFilesPattern("sql/next", /.*[0-9]{8}.sql$/)
        const preamble = fs.existsSync("sql/preamble.sql") ? fs.readFileSync("sql/preamble.sql") : ""
        fs.writeFileSync(`sql/next/${version}.sql`, preamble)
        console.log("NEW DB VERSION:", version)
    } catch (e) {
        console.error(e)
    }
}

export interface GenerateOptions {
    json: Partial<WriteJsonOptions>
    ts: Partial<WriteLibraryOptions>
}

/**
 * Generates the TS access layer.
 * 
 * This involves parsing the DB files. We will never generate them by connecting
 * to a real DB since that goes against the basic concept of compilation.
 * 
 * @param from Target version or alias.
 */
export async function generate(version: string, options: Partial<GenerateOptions> = {}): Promise<void> {
    const opts: GenerateOptions = {
        json: {},
        ts: {},
        ...options
    }
    try {
        const schema = generateDBSchema(version)
        writeJSON(schema, opts.json)
        writeLibrary(schema, opts.ts)
    } catch (e) {
        console.error(e)
    }
}

export * from './interfaces'
export const string_utils = stringUtils
export default { init, migrate, release, generate, string_utils, config, tools, lib }