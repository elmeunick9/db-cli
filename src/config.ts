type AppMode = "development"|"production"
const appMode : AppMode[]  = ["development", "production"]

export interface Config {
    db: {
        connection: {
            host: string
            database: string
            port: number
            ssl?: boolean
        }
        roles: {
            [role: string]: {
                user: string
                password: string
            }
        },
        prefix: string
        publicSchema: string
    }
    app: {
        name: string
        version: string
        description?: string
        source?: string
        mode: AppMode
        mock: boolean
        verbose: boolean
    }
    isDevelopment: () => boolean
    isVerbose: () => boolean
}


const oneOf = <T>(value: unknown, options: T[]): T => options.some(x => x === value) ? value as T : undefined

const config : Config = {
    db: {
        connection: {
            host:       process.env.DB_HOST                 || 'localhost',
            database:   process.env.DB_NAME                 || 'db',
            port:       parseInt(process.env.DB_PORT)       || 5432,
            ssl:        process.env.DB_SSL ? process.env.DB_SSL === 'true' : true
        },
        roles: {
            api: {
                user:       process.env.DB_USER             || 'api',
                password:   process.env.DB_PASS             || '0000',
            },
            sa: {
                user:       process.env.DB_SA_USER          || 'postgres',
                password:   process.env.DB_SA_PASS          || '0000',
            }
        },
        prefix:         process.env.DB_SCHEMA_PREFIX        || '',
        publicSchema:   process.env.DB_SCHEMA_PUBLIC        || 'public'
    },
    app: {
        name:           process.env.npm_package_name,
        version:        process.env.npm_package_version,
        mode:           oneOf(process.env.NODE_ENV, appMode)|| 'development',       // development | production
        mock:           Boolean(process.env.APP_MOCK)       || false,               // Allows disabling the server for running test
        verbose:        Boolean(process.env.APP_VERBOSE)    || true
    },
    isDevelopment: function(): boolean { return this.app.mode === 'development' },
    isVerbose: function(): boolean { return this.app.verbose && this.app.mode === 'development' }
}

export default config

process.env.DB_PASS = ''
process.env.DB_SA_PASS = ''
