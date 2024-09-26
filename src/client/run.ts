import { IClient } from "../interfaces.js"
import fs from "../tools/index.js"

export async function executeSqlFile(client: IClient, filepath: string, prefix: string[] = [], suffix: string[] = [], split=true): Promise<void> {
    const file = fs.loadSQLFile(filepath, { split })

    for (const sql of prefix) {
        await client.query(sql)
    }
    for (const sql of file) {
        await client.query(sql)
    }
    for (const sql of suffix) {
        await client.query(sql)
    }
}
