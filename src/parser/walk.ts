import { ASTNode } from "./parser"

export function walk(
    ast: ASTNode, 
    before: (before: ASTNode, path: string[]) => boolean|void = ():boolean => false, 
    after:  (before: ASTNode, path: string[]) => boolean|void = ():boolean => false, 
    path: string[] = []
): void {
    path = [...path, ast.name]
    if (before(ast, path)) return
    for (const child of ast.children) {
        walk(child, before, after, path)
    }
    if (after(ast, path)) return
}

export function print(ast: ASTNode): void {
    walk(ast, x => { delete x["parent"] })
}

export function queryNode(ast: ASTNode, selector: string): ASTNode {
    const selectorList = selector.split(' ')
    if (selectorList.length == 0 || selectorList.length > 1) {
        throw {message: `selector must by a single name`}
    }

    let found = null
    walk(ast, (node):boolean|void => {
        if (node.name == selectorList.at(-1)) {
            found = node
            return true
        }
    })
    return found
}

export function query(ast: ASTNode, selector: string): ASTNode[] {
    const selectorList = selector.split(' ');
    if (selectorList.length === 0) return [];

    const found = [];
    walk(ast, (node, path) => {
        if (node.name === selectorList[selectorList.length - 1]) found.push({ node, path });
    });

    return found.filter(({ path }) => {
        for (const name of selectorList) {
            if (!path.includes(name)) return false;
        }
        return true;
    }).map(({ node }) => node);
}