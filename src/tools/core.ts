import * as fs from './fs.js'
import { DB_ERROR } from '../errors.js'
import { generateDBSchema } from '../generator/index.js'
import { prefixSchema, unPrefixSchema, unquote } from './string_utils.js'

/**
 * Converts aliases to actual DB Versions. If you use a string in a version
 * format (YYYYMMDD) it ensures the version exists in source control and returns
 * it, otherwise it errors.
 * 
 * @param name A version literal or alias
 * @returns The actual version literal
 */
export function findVersion(name: string): string {
    const versionList = fs.listDBVersions().filter(x => x !== 'next').map(x => parseInt(x))

    const findLatestDBVersion = (): string => {
        if (versionList.length === 0) {
            console.error("Requested 'latest', but no release has been found!")
            throw DB_ERROR.INVALID_VERSION
        }
        return Math.max(...versionList).toString()
    }

    if (name === 'next')    return 'next'
    if (name === 'latest')  return findLatestDBVersion()
    if (typeof name == 'number') {
        if (versionList.find(x => x == name)) return name
        throw DB_ERROR.INVALID_VERSION
    }
    if (name.match(/^[1-9][0-9]{3}[0-1][0-9][0-3][0-9]$/g) && versionList.find(x => x == parseInt(name))) {
        return name
    }
    throw DB_ERROR.INVALID_VERSION
}

export interface SQLFileMatchFilter {
    schema?: string
    type?: 'schema'|'version'|'insert'|'snapshot'|'default'|'function'
    version?: string
    allowMultiple?: boolean
}

/**
 * Given a filter find all files of a schema that match it.
 * 
 * @param filter 
 * @returns 
 */
export function findSQLFiles(filter: SQLFileMatchFilter): string[] {
    filter = {
        schema: 'public',
        type: 'schema',
        version: 'next',
        ...filter
    }

    filter.schema = unPrefixSchema(filter.schema)

    const version = findVersion(filter.version)
    const basePath = `sql/${version}`
    const files = fs.readDirectoryRecursive(basePath)
        .filter(path => {
            const dirs = path.split('/').slice(1, -1)
            const filename = path.split('/').pop()

            if (!filename.endsWith('.sql')) return false

            if (filter.type === 'version') {
                if (filename.match(/^\d{8}\.sql$/)[0]) return true
                return false
            }

            // const isDefaultSchema   = !dirs[0] && filter.schema === 'public' && filename === `${filter.type}.sql`
            // const isFullName        = !dirs[0] && filename === `${filter.schema}.${filter.type}.sql`
            const matchClass           = filter.allowMultiple && filename.endsWith(`.${filter.type}.sql`)        
            const isFullInDirectory    = dirs[0] === filter.schema && (filename === `${filter.type}.sql` || matchClass)

            return isFullInDirectory
        })
        .map(path => basePath + path)
    
    return files
}

/**
 * @returns A list of each schema name.
 */
export function findSQLSchemas(version: string, sorted = false, prefix = ""): string[] {
    if (!sorted) {
        const names = fs.readDirectoryRecursive('sql/' + findVersion(version))
        .filter(path => path.endsWith('schema.sql'))
        .map(path => {
            const dirs = path.split('/').slice(1, -1)
            const filename = path.split('/').pop()

            if (filename.split('.').length === 3) return filename.split('.')[0]
            if (filename === 'schema.sql' && dirs.length === 1) return dirs[0]
            if (filename === 'schema.sql' && dirs.length === 0) return 'public'
            return undefined
        })
        .filter(x => x !== "public")

        return [prefixSchema("public", prefix), ...new Set(names)].map(x => `${prefixSchema(x, prefix)}`)
    }

    const dependencyMap = createSQLSchemaDependencyMap(version, prefix)

    // Sort the original names based on dependency DAG (Directed Acyclic Graph)
    // The O(n^3) algorithm is slow but its ok (we assume n=30 max)
    // We detect a cycle if after a pass final has not changed.
    const original = [...dependencyMap.keys()]
    const final = original.filter(x => dependencyMap.get(x).length == 0)
    let count = final.length
    while (final.length < original.length) {
        const toPush = original
            .filter(x => !final.includes(x))
            .filter(x => dependencyMap.get(x).every(
                (y: string) => final.includes(y)
            )
        )

        final.push(...toPush)

        if (count == final.length && final.length < original.length) {
            console.error("At: ", original.filter(x => !final.includes(x)))
            throw new Error("A schema dependency cycle was detected! Resolve it and try again.")
        } else {
            count = final.length
        }
    }

    return final
}


/**
 * @param version The version to use
 * 
 * @returns A list dependent schemas.
 */
export function createSQLSchemaDependencyMap(version: string, prefix = ""): Map<string, string[]> {
    const getTablePath = (input: string, schemaName: string): { schemaName: string, tableName: string } => input.includes('.')
        ? { schemaName: unquote(input.split('.')[0]), tableName: unquote(input.split('.')[1]) }
        : { schemaName: schemaName, tableName: unquote(input) }

    const out = new Map<string, string[]>()
    const schemaDB = generateDBSchema(version)
    for (const schemaName of Object.keys(schemaDB)) {
        const schema = schemaDB[schemaName]
        const deps = []
        for (const tableName of Object.keys(schema.tables)) {
            const table = schema.tables[tableName]
            const tableDeps = table.references
                .map(x => x.destination.table)
                .map(x => getTablePath(x, schemaName).schemaName)
                .filter(x => x != schemaName)
            deps.push(...tableDeps)
        }
        out.set(`${prefixSchema(schemaName, prefix)}`, deps.map(x => `${prefixSchema(x, prefix)}`))
    }

    return out
}
