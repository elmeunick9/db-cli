export * from "./init.js"
export * from "./run.js"

/**
 * Template literal tag. Doesn't do anything, use it just as a marker for SQL
 * strings.
 */
export const sql = (s: TemplateStringsArray, ...args: unknown[]): string => {
    const list = [s[0]]
    for (let i = 0; i < args.length; i++) {
        list.push(`${args[i]}`, s[i + 1])
    }
    return list.join('')
}