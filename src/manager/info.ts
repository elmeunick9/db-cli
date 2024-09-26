import { prefixSchema } from '../tools/string_utils.js'
import { IClient, Options } from '../interfaces.js'

export interface MetaInfo {
    'version': string
    'description': string
    'creation-date': string
    'source': string
}

export interface MetaState {
    migration: "ready"|"in-progress"
}

export async function getMetaByKey(client: IClient, opts: Options, key: string): Promise<MetaInfo|MetaState> {
    await client.connect({ ...opts.db.roles.sa })
    const response = await client.query(`SELECT "value" FROM "${prefixSchema(opts.db.publicSchema, opts.db.prefix)}"."meta" WHERE "key" = $1;`, [key])
    const result = response[0]?.rows[0]?.value ?? {}
    await client.end()
    return result
}

export async function setMetaByKey(client: IClient, opts: Options, key: string, value: MetaInfo|MetaState): Promise<void> {
    await client.connect({ ...opts.db.roles.sa })
    await client.query(`UPDATE "${prefixSchema(opts.db.publicSchema, opts.db.prefix)}"."meta" SET "value" = $2 WHERE "key" = $1;`, [key, value])
    await client.end()
}
