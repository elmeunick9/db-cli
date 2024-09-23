import pg from 'pg'
import config, { Config } from "../config.js"
import fs from "../tools/index.js"

export async function executeSqlFile(filepath: string, connection: Config['db']['connection']|pg.Client = config.db.connection, prefix: string[] = [], suffix: string[] = []): Promise<void> {
    const isClient = "connect" in connection
    const client = isClient ? connection : new pg.Client(connection)
    const file = fs.loadSQLFile(filepath)

    if (config.isVerbose()) {
        console.log("Executing SQL:", filepath)
    }

    if (!isClient) await client.connect()
    for (const sql of prefix) {
        await client.query(sql)
    }
    for (const sql of file) {
        await client.query(sql)
    }
    for (const sql of suffix) {
        await client.query(sql)
    }
    if (!isClient) await client.end()
}
