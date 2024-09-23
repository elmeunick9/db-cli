import { sql as q } from './index.js'

describe('Template Literal Marker `q`', () => {

    it('On expression returns same expression with no changes.', () => {
        expect(q`ab${"c"}d${"e" + "f"}`).toBe("abcdef")
        expect(q`abcdef`).toBe("abcdef")
        expect(q`${"ab"}${"c"}d${"e" + "f"}`).toBe("abcdef")
    })

})