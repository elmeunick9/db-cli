import { sql } from "../client/index.js"
import config from "../config.js"
import pg from 'pg'

/**
 * Deletes the DB if exists, then creates a new DB with the same name.
 * 
 * This function forcefully closes all connections to the DB beforehand. Should
 * only be used on development mode, since it will wipe out all data.
 */
export async function createCleanDB(): Promise<void> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa, database: 'postgres'})

    await client.connect()

    if (config.isDevelopment()) {
        await client.query(sql`
            SELECT pg_terminate_backend(pg_stat_activity.pid)
            FROM pg_stat_activity
            WHERE datname = '${config.db.connection.database}' AND pid <> pg_backend_pid();
        `)
        await client.query(sql`DROP DATABASE IF EXISTS ${config.db.connection.database};`)
    }
       
    await client.query(sql`CREATE DATABASE ${config.db.connection.database};`)
    await client.query(sql`ALTER DATABASE ${config.db.connection.database} SET TIMEZONE TO 'UTC';`)
    await client.end()
}

/**
 * Creates a user for accessing the DB. By default every role configured should 
 * have it's user. If no arguments are provided, creates the API user.
 * 
 * @param user 
 * @param pass 
 */
export async function createUser(user = config.db.roles.api.user, pass = config.db.roles.api.password): Promise<void> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})
    await client.connect()
    await client.query(sql`DROP USER IF EXISTS ${user};`)
    await client.query(sql`CREATE USER "${user}" WITH PASSWORD '${pass}';`)
    await client.end()
}

/**
 * Creates one user per each role defined in config.db.roles except SA.
 */
export async function createUsers(): Promise<void> {
    for (const [name, role] of Object.entries(config.db.roles)) {
        if (name == "sa") continue
        if (role.user === config.db.roles.sa.user) continue
        await createUser(role.user, role.password)
    }
}
