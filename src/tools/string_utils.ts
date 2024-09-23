import config from "../config"

export function prefixSchema(schema: string, prefix = config.db.prefix): string {
    return (config.db.prefix == null || config.db.prefix.length < 2 || schema.startsWith(prefix) || schema.startsWith("o_")) ? schema : `${prefix}${schema}`
}

export function unPrefixSchema(schema: string, prefix = config.db.prefix): string {
    return (config.db.prefix == null || config.db.prefix.length < 2 || !schema.startsWith(prefix)) ? schema : schema.slice(prefix.length)
}

export const quote = (s: string, c: "'" | '"' = '"'): string => {
    const single = s.at(0) == "'" && s.at(-1) == "'"
    const double = s.at(0) == '"' && s.at(-1) == '"'
    if (!single && !double) return `${c}${s}${c}`
    return s
}

export const unquote = (s: string): string => {
    if (!s || s.length < 2) return s
    if (s.at(0) == '"' && s.at(-1) == '"') return s.slice(1, -1)
    if (s.at(0) == "'" && s.at(-1) == "'") return s.slice(1, -1)
    return s
}

const schemaIdentifierRegexPattern = /(?<!\.)"[a-z0-9_]+"\./g;
export const replaceSchemaName = (text: string): string => {
    let result = "";
    let start = 0;
    const matches = [...text.matchAll(schemaIdentifierRegexPattern)];

    for (const match of matches) {
        // Append non-matching portion of the query
        result += text.slice(start, match.index);

        // Check if the match is within a string literal
        const prefixSingleQuoteCount = text.slice(0, match.index).split("'").length - 1;
        if ((prefixSingleQuoteCount % 2) === 0 && !match[0].startsWith('"' + config.db.prefix)) {
            // Perform substitution if not within a string literal
            const name = match[0].slice(1, -2)
            result += match[0].replace(match[0], `"${prefixSchema(name)}".`);
        } else {
            // Append the original match if within a string literal
            result += match[0];
        }

        start = match.index + match[0].length;
    }

    result += text.slice(start);
    return result;
}

export function toPascalCase(input: string): string {
    const words = input.split(/_|-| |(?<=[a-z])(?=[A-Z])/)
    return words.map(word => word[0].toUpperCase() + word.slice(1).toLowerCase()).join("")
}

export function toSnakeCase(input: string): string {
    const words = input.split(/_|-| |(?<=[a-z])(?=[A-Z])/)
    return words.map(word => word.toLowerCase()).join("_")
}

export function toCamelCase(input: string): string {
    const pascalCase = toPascalCase(input)
    return pascalCase[0].toLowerCase() + pascalCase.slice(1)
}

export function toKebabCase(input: string): string {
    const words = input.split(/_|-| |(?<=[a-z])(?=[A-Z])/)
    return words.map(word => word.toLowerCase()).join("-")
}

export function toScreamingCase(input: string): string {
    const words = input.split(/_|-| |(?<=[a-z])(?=[A-Z])/)
    return words.map(word => word.toUpperCase()).join("_")
}

type Transformer = (input: string) => string;

export function transformObjectKeys(obj: Record<string, unknown>, transformer: Transformer): unknown {
    const result: Record<string, unknown> = {};

    for (const key in obj) {
        if (Object.prototype.hasOwnProperty.call(obj, key)) {
            const modifiedKey = transformer(key);
            result[modifiedKey] = obj[key];
        }
    }

    return result;
}

type SnakeToCamel<S extends string> = S extends `${infer First}_${infer Rest}`
    ? `${First}${Capitalize<SnakeToCamel<Rest>>}`
    : S

type TransformKeysSnakeToCamel<T> = {
    [K in keyof T as SnakeToCamel<K & string>]: T[K];
}

export function transformObjectKeysSnakeToCamel<T>(obj: T): TransformKeysSnakeToCamel<T> {
    return transformObjectKeys(obj as Record<string, unknown>, toCamelCase) as TransformKeysSnakeToCamel<T>
}

// We've only done it type safe with one for now!

type ReplaceUndefinedWithNull<T> = T extends undefined ? null : T;
type RemoveUndefined<T> = {
  [P in keyof T]-?: ReplaceUndefinedWithNull<T[P]>
}

export function transformUndefinedToNull<T>(input: T): RemoveUndefined<T> {
    const result: RemoveUndefined<T> = {} as RemoveUndefined<T>;
    for (const key in Object.keys(input)) {
        if (Object.prototype.hasOwnProperty.call(input, key)) {
            const value = input[key];
            result[key] = value === undefined ? null : value as T[keyof T];
        }
    }
    return result;
}