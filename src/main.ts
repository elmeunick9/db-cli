#!/usr/bin/env node
import db from './index.js'
import { Command } from 'commander'
import config from './config.js'
const program = new Command()

program
    .name('NORM')
    // Using `process.env.npm_package_*` WILL break the build.
    .description("Not an Object Relational Mapper")
    .version("0.0.0")

program.command('init')
    .summary('creates and initializes the DB')
    .description(`Initializes the DB when on development mode. It will delete and recreate the DB to the version specified.`)
    .argument('[version]', 'Version to use', config.isDevelopment() ? 'next' : 'latest')
    .action(async (v) => await db.init(v))

program.command('migrate')
    .summary('migrates to version specified')
    .description(`Migrates the DB from the current version to the version specified, by default "latest" in production or "next" in development.`)
    .argument('[version]', 'Version to use', config.isDevelopment() ? 'next' : 'latest')
    .action(async (v) => await db.migrate(v))

program.command('release')
    .summary('creates a new DB version')
    .description(`Creates the appropriate folders for a new DB version.`)
    .argument('[version]', 'New version number', new Date().toISOString().slice(0, 10).replace(/-/g, ''))
    .action(async (v) => await db.release(v))

program.command('generate')
    .summary('generates a TS library to access the DB.')
    .description(`Creates a data access layer for TypeScript and a JSON schema.`)
    .argument('[version]', 'Version to use', config.isDevelopment() ? 'next' : 'latest')
    .option('-j, --json <file>', 'JSON schema file', 'generated/db-schema.json')
    .option('-t, --ts <file>', 'TS file', 'generated/db-cli.ts')
    .action(async (v) => await db.generate(v))

program.parse()


