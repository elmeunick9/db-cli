import config from './config'
import manager from './manager'
import tools from './tools/index.js'
import { DB_ERROR } from './errors.js'
import { DatabaseError } from 'pg'
import { MetaInfo } from './manager/info.js'
import fs from 'fs'
import { generateDBSchema, writeJSON, writeLibrary } from './generator'
import * as stringUtils from './tools/string_utils';

/**
 * Initializes the DB when on development mode. It will delete and recreate the DB
 * to the version specified. 
 * 
 * @param from 
 */
export async function init(from = config.isDevelopment() ? 'next' : 'latest'): Promise<void> {
    try {
        if (config.isDevelopment()) {
            console.log("Cleaning DB.")
            await manager.createCleanDB()
        }
        await manager.createUsers()
        for (const schema of tools.findSQLSchemas(from, true)) {
            await manager.createSchema(schema, from)
        }
        for (const schema of tools.findSQLSchemas(from, true)) {
            await manager.initializeSchema(schema, from)
        }
        
        const info = await manager.getMetaByKey('info') as MetaInfo
        if (!info.version) throw DB_ERROR.CORRUPTED

        console.log("DB initialized.")
    } catch (e) {
        if ((e as DatabaseError).code === '42P04') {
            console.error("DB already exists! Aborted.")
        } else {
            console.error(e)
            console.info(JSON.stringify(e))
        }
        process.exit(1)
    }
}

/**
 * Migrates the DB to the version specified. 
 * @param from Target version or alias.
 */
export async function migrate(to = config.isDevelopment() ? 'next' : 'latest'): Promise<void> {
    try {
        const info = await manager.getMetaByKey('info') as MetaInfo
        if (!info.version) throw DB_ERROR.CORRUPTED
    
        const plan = manager.createMigrationPlan(info.version, to)
        let current = info.version
        if (plan.length === 0) {
            console.log("Already on last release, skipping migration.")
        }
        for (const step of plan) {
            await manager.migrateStep(current, step)
            current = step
        }
    } catch (e) {
        console.error(e)
        process.exit(1)
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
        process.exit(1)
    }
}

/**
 * Generates the TS access layer.
 * 
 * This involves parsing the DB files. We will never generate them by connecting
 * to a real DB since that goes against the basic concept of compilation.
 * 
 * @param from Target version or alias.
 */
export async function generate(version: string, jsonDest?: string, tsDest?: string): Promise<void> {
    try {
        const schema = generateDBSchema(version)
        writeJSON(schema, jsonDest)
        writeLibrary(schema, tsDest)
    } catch (e) {
        console.error(e)
        process.exit(1)
    }
}

export const string_utils = stringUtils
export default { init, migrate, release, generate, string_utils }