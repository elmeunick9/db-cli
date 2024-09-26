import { sql } from "../client/index"
import { DB_ERROR } from "../errors"
import { IClient, InitOptions } from "../interfaces"

/**
 * Deletes the DB if exists, then creates a new DB with the same name.
 * 
 * This function forcefully closes all connections to the DB beforehand. Should
 * only be used on development mode, since it will wipe out all data.
 */
export async function createCleanDB(client: IClient, opts: InitOptions): Promise<void> {
    if (!opts.db.roles.sa) {
        opts.logger.error('Missing role config for `sa`')
        throw new Error(DB_ERROR.MISSING_OPTION)
    }
    await client.connect({...opts.db.roles.sa, database: 'postgres'})

    if (opts.terminate) {
        await client.query(sql`
            SELECT pg_terminate_backend(pg_stat_activity.pid)
            FROM pg_stat_activity
            WHERE datname = '${opts.db.connection.database}' AND pid <> pg_backend_pid();
        `, [])
        await client.query(sql`DROP DATABASE IF EXISTS ${opts.db.connection.database};`)
    }
       
    await client.query(sql`CREATE DATABASE ${opts.db.connection.database};`)
    await client.query(sql`ALTER DATABASE ${opts.db.connection.database} SET TIMEZONE TO 'UTC';`)
    await client.end()
}

/**
 * Creates a user for accessing the DB. By default every role configured should 
 * have its user. If no arguments are provided, creates the API user.
 */
export async function createUser(client: IClient, opts: InitOptions, role: string): Promise<void> {
    if (!opts.db.roles.sa) {
        opts.logger.error('Missing role config for `sa`')
        throw new Error(DB_ERROR.MISSING_OPTION)
    }
    if (!opts.db.roles[role]) {
        opts.logger.error(`Missing role config for ${role}`)
        throw new Error(DB_ERROR.MISSING_OPTION)
    }
    const user = opts.db.roles[role].user
    const pass = opts.db.roles[role].password
    await client.connect({...opts.db.roles.sa, database: 'postgres'})
    await client.query(sql`DROP USER IF EXISTS ${user};`)
    await client.query(sql`CREATE USER "${user}" WITH PASSWORD '${pass}';`)
    await client.end()
}

/**
 * Creates one user per each role defined in config.db.roles except SA.
 */
export async function createUsers(client: IClient, opts: InitOptions): Promise<void> {
    for (const [name, role] of Object.entries(opts.db.roles)) {
        if (name == "sa") continue
        if (role.user === opts.db.roles.sa.user) continue
        await createUser(client, opts, name)
    }
}
