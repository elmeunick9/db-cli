import { IClient } from "../interfaces.js"
import { parseCommand } from "../tools/fs.js"
import fs from "../tools/index.js"

export type CommandExecutionSet = {[key: string]: (client: IClient, args: string[]) => Promise<void>}

export async function executeSqlFile(client: IClient, filepath: string, prefix: string[] = [], suffix: string[] = [], split=true, commands: CommandExecutionSet = {}): Promise<void> {
    const file = fs.loadSQLFile(filepath, { split, mode: Object.keys(commands).length > 0 ? 'both' : 'sql' }, client.settings().prefix)

    for (const sql of prefix) {
        await client.query(sql)
    }
    for (const line of file) {
        if (line.startsWith("-->")) {
           const command = parseCommand(line) 
           if (commands[command.name]) {
               await commands[command.name](client, command.args)
           }
        } else {
            await client.query(line)
        }
    }
    for (const sql of suffix) {
        await client.query(sql)
    }
}
