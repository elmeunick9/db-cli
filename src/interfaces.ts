/* eslint-disable @typescript-eslint/no-explicit-any */

import type { Config } from "./config"
import { SQLQuery } from "./generator/lib/queryBuilder"

export type MessageName =
  | 'parseComplete'
  | 'bindComplete'
  | 'closeComplete'
  | 'noData'
  | 'portalSuspended'
  | 'replicationStart'
  | 'emptyQuery'
  | 'copyDone'
  | 'copyData'
  | 'rowDescription'
  | 'parameterDescription'
  | 'parameterStatus'
  | 'backendKeyData'
  | 'notification'
  | 'readyForQuery'
  | 'commandComplete'
  | 'dataRow'
  | 'copyInResponse'
  | 'copyOutResponse'
  | 'authenticationOk'
  | 'authenticationMD5Password'
  | 'authenticationCleartextPassword'
  | 'authenticationSASL'
  | 'authenticationSASLContinue'
  | 'authenticationSASLFinal'
  | 'error'
  | 'notice'

interface NoticeOrError {
    message: string | undefined
    severity: string | undefined
    code: string | undefined
    detail: string | undefined
    hint: string | undefined
    position: string | undefined
    internalPosition: string | undefined
    internalQuery: string | undefined
    where: string | undefined
    schema: string | undefined
    table: string | undefined
    column: string | undefined
    dataType: string | undefined
    constraint: string | undefined
    file: string | undefined
    line: string | undefined
    routine: string | undefined
  }
  
  export class DatabaseError extends Error implements NoticeOrError {
    public severity: string | undefined
    public code: string | undefined
    public detail: string | undefined
    public hint: string | undefined
    public position: string | undefined
    public internalPosition: string | undefined
    public internalQuery: string | undefined
    public where: string | undefined
    public schema: string | undefined
    public table: string | undefined
    public column: string | undefined
    public dataType: string | undefined
    public constraint: string | undefined
    public file: string | undefined
    public line: string | undefined
    public routine: string | undefined
    constructor(
      message: string,
      public readonly length: number,
      public readonly name: MessageName
    ) {
      super(message)
    }
  }

export interface IFieldDef {
    name: string;
    tableID: number;
    columnID: number;
    dataTypeID: number;
    dataTypeSize: number;
    dataTypeModifier: number;
    format: string;
}

export interface IQueryResult {
    rows: any[]
    rowCount: number|null
    fields: IFieldDef[]
    command: string
    oid: number
}

export interface IClientOptions {
    database: string
    host: string
    port: number
    user: string
    password: string
    ssl?: boolean
    prefix?: string
}

export interface IClient {
    settings(): IClientOptions
    connect(options?: Partial<IClientOptions>): Promise<void>
    query(sql: string, values?: any[]): Promise<IQueryResult[]>
    query(sql: SQLQuery): Promise<IQueryResult[]>
    end(): Promise<void>
}

export interface ILogger {
    log(...message: string[]): void
    error(...message: string[]): void
    warn(...message: string[]): void
    info(...message: string[]): void
}

export interface Options {
    db: Config['db']
    development: boolean
    terminate: boolean
    logger: ILogger
}

export interface InitOptions extends Options {
    from: string
    clean: boolean
    createUsers: boolean
    meta: {
        description: string
        source: string
    }
}

export interface MigrationOptions extends InitOptions {
    to: string
}