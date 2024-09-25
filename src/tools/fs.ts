import fs from 'fs'
import { replaceSchemaName } from './string_utils'

export interface SourceOptions {
    encoding?: 'utf8'|'utf16le'|'latin1'|'base64'|'base64url'|'hex'|'ascii'|'ucs2'|'ucs-2'
    type?: 'PATH'|'MEMORY'
    split?: boolean
}

export interface CopyOptions {
    /**
     * Path prefix to exclude from the copy.
     */
    excludePrefix?: string
}

/**
 * Reads an SQL file from disk, splits it into commands and cleans comments,
 * new line characters and blanks.
 * 
 * @param filepath 
 * @returns A list of SQL commands.
 */
export function loadSQLFile(file: string|Buffer, options: SourceOptions = {}, withPrefix = true): string[] {
    const data : string = options.type === 'MEMORY' 
        ? file.toString()
        : fs.readFileSync(file, { encoding: options.encoding ?? 'utf8' })

    const p_data = (options.split === false) ? [data] : data
        .replace(/(\r\n|\n|\r)/gm, '\n')
        .split(/(?=\n|--)/g)
        .filter(x => !x.startsWith('--'))
        .join('')
        .replace(/\s+/g, ' ')
        .split(';')
        .map(x => x.trim())
        .filter(x => x.length != 0)
        
    if (withPrefix) return p_data.map(x => replaceSchemaName(x))
    else return p_data
}

/**
 * Returns a list of all files in a given directory.
 * 
 * @param basePath 
 * @returns Array of strings containing the relative path of each file.
 */
export function readDirectoryRecursive(basePath: string): string[] {
    const walk = (
        basePath: string, 
        mainPath: string, 
        exec: (path: string) => unknown
    ): void => {
        fs.readdirSync(basePath + mainPath).forEach(node => {
            const path = mainPath + "/" + node
            fs.statSync(basePath + "/" + path).isDirectory() 
                ? walk(basePath, path, exec) 
                : exec(path)
        })
    }

    const files: string[] = []
    walk(basePath, '', path => files.push(path))
    return files
}

/**
 * Copies a folder to another path.
 * 
 * @param src Source path of the folder.
 * @param dest Destination path of the folder.
 * @param options 
 * @returns 
 */
export function copyDirectoryRecursive(src: string, dest: string, options: CopyOptions = {}): void {
    const isDirectory = fs.existsSync(src) && fs.statSync(src).isDirectory()
    const isPrefixToExclude = options.excludePrefix && src.startsWith(options.excludePrefix)
    
    if (isPrefixToExclude) return

    if (isDirectory) {
        if (!fs.existsSync(dest)) fs.mkdirSync(dest);
        fs.readdirSync(src).forEach(
            path => copyDirectoryRecursive(src + "/" + path, dest + "/" + path, options)
        )
    } else {
        fs.copyFileSync(src, dest)
    }
}

/**
 * Removes all files in a given directory whose path matches the given RegEx.
 * @param src Path where to look at, only one level depth.
 * @param regex Expression, if matched the file is removed.
 */
export function removeFilesPattern(src: string, regex: RegExp): void {
    fs.readdirSync(src).forEach(
        path => path.match(regex) ? fs.unlinkSync(src + '/' + path) : null
    )
}

/**
 * Retrieves the list of available DB versions from the sql folder.
 * @returns A list of string representing versions of your DB.
 */
export function listDBVersions(): string[] {
    try         { return [ ...fs.readdirSync('sql') ] } 
    catch (e)   { return [ 'next' ] }  
}

export type MigrationGraph = MigrationGraphNode[]
export interface MigrationGraphNode {
    version: string
    targets: string[]

    // Used by the shortest distance calculation algorithm. 
    path: string[]|null
}

export function createMigrationGraph(): MigrationGraph {
    const extractTargets = (DBVersion: string): string[] => {
        try {
            const files = fs.readdirSync(`sql/${DBVersion}`)
            return files
                .filter(x => x.match(/^[1-9][0-9]{3}[0-1][0-9][0-3][0-9]\.sql$/g))
                .map(x => x.replace('.sql', ''))
        } catch (e) {
            return []
        }
    }

    return listDBVersions().map(version => ({
        version,
        targets: extractTargets(version),
        path: null
    }))
}

