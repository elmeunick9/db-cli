/* eslint-disable no-useless-escape */

export interface Token {
    name: string
    value: string
    position: number
}

export function tokenize(sql: string): Token[] {
    const tokens: Token[] = []
    let position = 0
    while(sql) {
        const def = {
            create_extension: sql.match(/^CREATE EXTENSION( IF NOT EXISTS)?/),
            create_table: sql.match(/^CREATE TABLE/),
            create_type: sql.match(/^CREATE TYPE/),
            create_domain: sql.match(/^CREATE DOMAIN/),
            create_index: sql.match(/^CREATE( UNIQUE)? INDEX( CONCURRENTLY)?/),
            check: sql.match(/^CHECK/),
            value_marker: sql.match(/^VALUE/),
            comparison_op: sql.match(/^(\<|\<\=|\<\>|\=|\>|\>\=|\|\||\!\!\=|\~\~|\!\~\~|\~|\~\*|\!\~|\!\~\*)/),
            arithmetic_op: sql.match(/^(\+|\-|\*|\/|\%)/),
            logical_op: sql.match(/^(AND|OR|NOT|IN|LIKE|BETWEEN|EXISTS|SOME|ANY|IS)/),
            identifier: sql.match(/^(\"[a-z0-9_]+\"|[a-z0-9_]+)(\.(\"[a-z0-9_]+\"|[a-z0-9_]+))*/),
            space: sql.match(/^ +/),
            semicolon: sql.match(/^;/),
            op_p: sql.match(/^\(/),
            cl_p: sql.match(/^\)/),
            sep: sql.match(/^\,/),
            null_constraint: sql.match(/^NULL|^NOT NULL/),
            default: sql.match(/^DEFAULT/),
            type: sql.match(/^(varchar(\ *\([0-9]+\))?|char(\ *\([0-9]+\))?|date|timestamptz|timestamp|integer|bigint|smallint|int2|int4|int8|int|float4|float8|float|real|double precision|timetz|time|text|uuid|boolean|numeric(\ *\(( *[0-9\,]+ *)+\))?|decimal(\ *\(( *[0-9\,]+ *)+\))?|money)/),
            value: sql.match(/^(now\(\)|[a-z0-9_]+\(\)|true|false|null|[0-9]+|\'(.*?)\')/),
            primary_key: sql.match(/^PRIMARY KEY/),
            foreign_key: sql.match(/^FOREIGN KEY/),
            references: sql.match(/^REFERENCES/),
            alias: sql.match(/^AS/),
            enum: sql.match(/^ENUM/),
            unique: sql.match(/^UNIQUE( NULLS NOT DISTINCT)?/),
            exclude: sql.match(/^EXCLUDE/),
            using: sql.match(/^USING/),
            on_delete: sql.match(/^ON DELETE (RESTRICT|CASCADE|NO ACTION|SET NULL|SET DEFAULT)/),
            on: sql.match(/^ON/),
            include: sql.match(/^INCLUDE/),
            where: sql.match(/^WHERE/),
            with: sql.match(/^WITH/),
            table_space: sql.match(/^TABLESPACE/),
            inherits: sql.match(/^INHERITS/),
            grant: sql.match(/^GRANT .*/),
            set: sql.match(/^SET .*/),
            alter_table: sql.match(/^ALTER TABLE .*/)
        }
        const skip = ["space", "semicolon"]
        const preference = {
            "-1": ["identifier"]
        }

        let token_preference = -9999
        let token = {name: "", value: "", position}
        Object.keys(def).forEach(x => {
            if (!def[x] || !def[x][0]) return
            const value = def[x][0]
            const pref = parseInt(Object.keys(preference)
                .map(y => preference[y].find((z: string) => z == x))
                .filter(y => y != null)[0] ?? "0")
            if (pref < token_preference) return
            else if (pref == token_preference) {
                token = def[x][0].length > token.value.length ? {name: x, value, position } : token
            } else {
                token = {name: x, value, position }
            }
            token_preference = pref
        })
        if (token.value == "") throw {
            message: `At ${position}, no token to parse ${tokens.at(-1) ? `after ${tokens.at(-1)}` : "" }
            Context: "${sql.length > 30 ? sql.slice(0, 50) + "..." : sql}"`,
            position
        }
        if (!skip.includes(token.name)) {
            tokens.push(token)
        }
        sql = sql.slice(token.value.length)
        position += token.value.length
    }
    return tokens
}