type AppMode = "development"|"production"

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

const config : Config = {
    db: {
        connection: {
            host: 'localhost',
            database: 'db',
            port: 5432,
            ssl: true
        },
        roles: {
            api: {
                user: 'api',
                password: '0000',
            },
            sa: {
                user: 'postgres',
                password: '0000',
            }
        },
        prefix: '',
        publicSchema: 'public'
    },
    app: {
        name:           'default',
        version:        '0.1.0',
        mode:           'development',       // development | production
        mock:           false,               // Allows disabling the server for running test
        verbose:        true
    },
    isDevelopment: function(): boolean { return this.app.mode === 'development' },
    isVerbose: function(): boolean { return this.app.verbose && this.app.mode === 'development' }
}

export default config
