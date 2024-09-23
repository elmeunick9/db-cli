import pg from 'pg'
import config from "../config.js"
import { prefixSchema } from '../tools/string_utils.js'

export interface MetaInfo {
    'version': string
    'description': string
    'creation-date': string
    'source': string
}

export interface MetaState {
    migration: "ready"|"in-progress"
}

export async function getMetaByKey(key: string): Promise<MetaInfo|MetaState> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    await client.connect()
    const response = await client.query(`SELECT "value" FROM "${prefixSchema("public")}"."meta" WHERE "key" = $1;`, [key])
    const result = response.rows[0]?.value ?? {}
    await client.end()
    return result
}

export async function setMetaByKey(key: string, value: MetaInfo|MetaState): Promise<void> {
    const client = new pg.Client({...config.db.connection, ...config.db.roles.sa})

    await client.connect()
    await client.query(`UPDATE "${prefixSchema("public")}"."meta" SET "value" = $2 WHERE "key" = $1;`, [key, value])
    await client.end()
}
