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

        it('Check function loading', () => {
            const file = `
CREATE OR REPLACE FUNCTION create_order(
    order_meta jsonb,
    order_items jsonb
) RETURNS bigint AS $$ 
DECLARE
    new_order_id bigint;
BEGIN
    INSERT INTO "order" (meta)
    VALUES (order_meta)
    RETURNING id INTO new_order_id;
    -- End transaction (implicit commit)
EXCEPTION
    -- Rollback and raise error in case of any exception
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Transaction failed: %', SQLERRM;
END;
$$ LANGUAGE plpgsql;`

            expect(fs.loadSQLFile(file, { type: 'MEMORY', split: false })).toMatchObject([file])
        })
    })
})