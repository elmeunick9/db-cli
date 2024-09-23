import { Token, tokenize } from "./tokenizer"

export interface LooseToken {
    name?: string
    value?: string
    position?: number
}

export interface ASTNode {
    name: string
    value: string
    position: number
    children: ASTNode[]
    parent?: ASTNode
}

export function parse(sql: string, options = { exhaustive: true }): ASTNode {
    const tokens = tokenize(sql)
    const ast = { name: "program", value: "", position: 0, children: [], parent: null }

    // const right = (cur: ASTNode, node: ASTNode): ASTNode => {
    //     cur.parent.children.push({...node, position: cur.position + node.value.length, parent: cur.parent })
    //     return cur.parent.children.at(-1)
    // }
    const down = (cur: ASTNode, node: ASTNode): ASTNode => {
        cur.children.push({...node, position: cur.position + node.value.length, parent: cur })
        return cur.children.at(-1)
    }
    const node = (obj: LooseToken): ASTNode => ({ name: "default", value: "", position: 0, children: [], parent: null, ...obj })
    const shift = (): Token => {
        const token = tokens.shift()
        if (!token) throw `Reached EOF!`
        return token
    }
    const next = (n = 1): Token => tokens[n - 1]
    const fail = (token: Token, msg: string = ""): void => {
        sql = sql.slice(token.position)
        if (msg == "") {
            throw new Error(`At ${token.position}, ${msg}
            Context: ${sql.length > 30 ? sql.slice(0, 50) + "..." : sql}`)
        }
        throw new Error(`At ${token.position}, no parse rule for "${token.name}"
            Context: ${sql.length > 30 ? sql.slice(0, 50) + "..." : sql}`)
    }
    const exhaust = (): void => {
        tokens.length = 0
    }

    function columnList(ast: ASTNode): ASTNode {
        let token = shift()
        if (token.name != "op_p") fail(token)
        
        while(next().name == "identifier") {
            down(ast, node(shift()))
            if (next().name == "sep" && next(2).name != "cl_p") shift()
        }

        token = shift()
        if (token.name != "cl_p") fail(token)

        return ast
    }

    function valueList(ast: ASTNode, onlyLabels = false): ASTNode {
        let token = shift()
        if (token.name != "op_p") fail(token)
        
        while(next().name == "value") {
            const isLabel = next().value.length >= 2 && next().value.at(0) == "'" && next().value.at(-1) == "'"
            if (onlyLabels && !isLabel) fail(shift())
            down(ast, node(shift()))
            if (next().name == "sep" && next(2).name != "cl_p") shift()
        }

        token = shift()
        if (token.name != "cl_p") fail(token)

        return ast
    }

    function excludeList(ast: ASTNode): ASTNode {
        let token = shift()
        if (token.name != "op_p") fail(token)
        
        while(next().name == "identifier") {
            down(ast, node(shift()))

            token = shift()
            if (token.name != "with") fail(token)
            
            token = shift()
            if (token.name != "comparison_op") fail(token)

            if (next().name == "sep" && next(2).name != "cl_p") shift()
        }

        token = shift()
        if (token.name != "cl_p") fail(token)

        return ast
    }

    function predicate(ast: ASTNode): ASTNode {
        return columnList(ast)
    }

    function arithmeticExpression(ast: ASTNode, initialValue: number): number {
        if (isNaN(initialValue)) fail(next(), 'not a number')

        let value = initialValue
        while (next().name == "arithmetic_op") {
            const expr = shift()
            const token = shift()
            if (token.name != "value") fail(token)
            
            const operand = Number(token.value)
            if (isNaN(operand)) fail(token, 'not a number')

            if (expr.value === "+") value += operand;
            if (expr.value === "-") value -= operand;
            if (expr.value === "*") value *= operand;
            if (expr.value === "/") value /= operand;
            if (expr.value === "%") value = value % operand;
        }

        return value
    }

    function column(ast: ASTNode): ASTNode {
        let token = shift()
        if (token.name == "identifier") down(ast, node(token))
        else fail(token)

        token = shift()
        if (token.name == "type") down(ast, node(token))
        else if (token.name == "identifier") down(ast, node({...token, name: "type_ref"}))
        else fail(token)

        if (next().name == "null_constraint") {
            down(ast, node(shift()))   
        }

        if (next().name == "default") {
            shift()
            token = shift()
            if (token.name == "null_constraint" && token.value == "NULL") {
                down(ast, node({...token, name: "default_value"}))
            } else if (token.name == "value") {
                if (next().name != "arithmetic_op") {
                    down(ast, node({...token, name: "default_value"}))
                } else if (!isNaN(Number(token.value))) {
                    const value = arithmeticExpression(ast, Number(token.value))
                    down(ast, node({...token, name: "default_value", value: value.toString()}))
                }
            }
            else fail(token)
        }

        return ast
    }

    function columnDefinitionSet(ast: ASTNode): ASTNode {
        let token = shift()
        if (token.name == "op_p") {
            while(next().name == "identifier") {
                column(down(ast, node({name: "column"})))
                if (next().name == "sep" && next(2).name != "cl_p") shift()
            }
            if (next().name == "primary_key") {
                token = shift()
                columnList(down(ast, node(token)))
                if (next().name == "sep" && next(2).name != "cl_p") shift()
            }
            while (next().name == "foreign_key") {
                token = shift()
                const foreign_key = down(ast, node(token))
                const source = down(foreign_key, node({name: "source"}))
                const dest = down(foreign_key, node({name: "dest"}))
                columnList(source)
                token = shift()
                if (token.name != "references") fail(token)
                token = shift()
                if (token.name != "identifier") fail(token)
                down(dest, node(token))
                columnList(down(dest, node({name: "columns"})))
                if (next().name == "on_delete") shift()
                if (next().name == "sep" && next(2).name != "cl_p") shift()
            }
            while (next().name == "unique") {
                token = shift()
                columnList(down(ast, node(token)))
                if (next().name == "sep" && next(2).name != "cl_p") shift()
            }
            while (next().name == "exclude") {
                token = shift()
                if (next().name == "using") {
                    token = shift()
                    token = shift()
                    if (token.name != "identifier") fail(token)
                }
                
                excludeList(down(ast, node(token)))

                if (next().name == "where") {
                    token = shift()
                    predicate(down(ast, node(token)))
                }

                if (next().name == "sep" && next(2).name != "cl_p") shift()
            }
            while (next().name == "check") {
                let p = 1
                while (p > 0) {
                    if (next().name == "cl_p") p--
                    if (next().name == "op_p") p++
                    if (next(2)) shift()
                }
                while (next().name == "sep") shift()
            }
            if (next().name == "cl_p") shift()
            else fail(next())
        }
        return ast
    }

    function createTable(ast: ASTNode): void {
        let token = shift()
        if (token.name == "identifier") {
            const table_name =  down(ast, node({name: "table_name"}))
                                down(table_name, node(token))
            const column_set =  down(ast, node({name: "column_set"}))

            columnDefinitionSet(column_set)

            if (next()?.name == "inherits") {
                token = shift()
                columnList(down(ast, node(token)))
            }
        }
    }

    function createDomain(ast: ASTNode): void {
        let token = shift()
        if (token.name == "identifier") down(ast, node(token))
    
        token = shift()
        if (token.name != "alias") fail(token)

        token = shift()
        if (token.name == "type") down(ast, node(token))

        if (next()?.name == "check") {
            down(ast, node({name: "skip", value: sql}))
            exhaust()
        }
    }

    function createType(ast: ASTNode): void {
        let token = shift()
        if (token.name == "identifier") down(ast, node(token))
    
        token = shift()
        if (token.name != "alias") fail(token)

        token = shift()
        if (token.name == "enum") valueList(down(ast, node(token)), true)
    }

    function program(ast: ASTNode): ASTNode {
        const token = shift()
        if (token.name == "create_extension") return down(ast, node({name: "skip", value: sql}))
        if (token.name == "grant") return down(ast, node({name: "skip", value: sql}))
        if (token.name == "set") return down(ast, node({name: "skip", value: sql}))
        if (token.name == "alter_table") return down(ast, node({name: "skip", value: sql}))
        if (token.name == "create_index") return down(ast, node({name: "skip", value: sql}))
        if (token.name == "create_type") {
            createType(down(ast, node(token)))
            if (tokens.length > 0) fail(next())
            return ast
        }
        if (token.name == "create_domain") {
            createDomain(down(ast, node(token)))
            if (tokens.length > 0) fail(next())
            return ast
        }
        if (token.name == "create_table") {
            createTable(down(ast, node(token)))
            if (tokens.length > 0) fail(next())
            return ast
        }
        throw fail(token)
    }

    program(ast)

    if (options.exhaustive && tokens.length > 0 && ast.children[0]?.name != "skip") {
        throw { message: `Error: Parse finished before reaching EOF`, ast }
    }

    return ast
}