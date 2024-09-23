import * as fs from './fs.js'

describe('FileSystem Related SQL Tools', () => {

    describe('loadSQLFile', () => {

        it('Check proper formatting', () => {
            const file = `
            INSERT 1; -- Some comment;;
            
            -- Comment
            INSERT 2;`
            
            expect(fs.loadSQLFile(file, { type: 'MEMORY' })).toMatchObject(["INSERT 1", "INSERT 2"])
        })

    })

})