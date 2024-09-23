import * as lib from './index'

describe('Generator Lib', () => {

    test('readByKey', async () => {
        lib.default.init()
        const market = await lib.generic.readByKey("public", "market", {cname: "NSY"})
        expect(market).toEqual({
            "close": "21:00:00",
            "cname": "NSY",
            "currency": "USD",
            "name": "NYSE: New York Stock Exchange",
            "open": "14:30:00",
        })
    })

    afterAll(async () => {
        await lib.client().end()
    }, 1000);

})