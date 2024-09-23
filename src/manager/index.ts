import * as lifecycle from './lifecycle.js'
import * as schema from './schema.js'
import * as info from './info.js'
import * as migration from './migration.js'

export default { ...lifecycle, ...schema, ...info, ...migration }

