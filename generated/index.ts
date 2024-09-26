
/* eslint-disable @typescript-eslint/prefer-namespace-keyword */
/* eslint-disable @typescript-eslint/no-namespace */
import type { kObject, ListOptions, ReadOptions, Options, Clause, IClient, DBSchema } from "../dist/types/generator/lib/index.d.ts"
import { lib } from "../dist/lib.mjs"
export * as lib from "../dist/lib.mjs"

// -- INTERFACES --
export namespace schema {
    export namespace Public {

        export type asset_type = "IDX"|"STK"|"FX"|"CFX"
        export interface MarketKey {
            cname: string
        }
        export interface MarketData {
            name?: string|null
            currency?: string
            open?: string|null
            close?: string|null
        }
        export interface Market extends MarketKey, MarketData {}
        export type MarketColumn = "cname"|"name"|"currency"|"open"|"close"

        export interface ProviderKey {
            cname: string
        }
        export interface ProviderData {
            name?: string|null
        }
        export interface Provider extends ProviderKey, ProviderData {}
        export type ProviderColumn = "cname"|"name"

        export interface ListingKey {
            symbol: string
        }
        export interface ListingData {
            type: asset_type
            name?: string|null
            market: string
            enabled?: boolean
            isin?: string|null
            sector?: string|null
            industry?: string|null
        }
        export interface Listing extends ListingKey, ListingData {}
        export type ListingColumn = "type"|"symbol"|"name"|"market"|"enabled"|"isin"|"sector"|"industry"

    }
}

const _info: DBSchema = {"public":{"tables":{"market":{"type":"table","name":"market","columns":[{"name":"cname","type":"varchar(8)","isNullable":false,"hasDefault":false},{"name":"name","type":"varchar(80)","isNullable":true,"hasDefault":false},{"name":"currency","type":"varchar(3)","isNullable":false,"hasDefault":true},{"name":"open","type":"time","isNullable":true,"hasDefault":false},{"name":"close","type":"time","isNullable":true,"hasDefault":false}],"key":["cname"],"references":[],"path":"\"public\".\"market\""},"provider":{"type":"table","name":"provider","columns":[{"name":"cname","type":"varchar(8)","isNullable":false,"hasDefault":false},{"name":"name","type":"varchar(80)","isNullable":true,"hasDefault":false}],"key":["cname"],"references":[],"path":"\"public\".\"provider\""},"listing":{"type":"table","name":"listing","columns":[{"name":"type","type":"asset_type","isNullable":false,"hasDefault":false},{"name":"symbol","type":"varchar(8)","isNullable":false,"hasDefault":false},{"name":"name","type":"varchar(80)","isNullable":true,"hasDefault":false},{"name":"market","type":"varchar(8)","isNullable":false,"hasDefault":false},{"name":"enabled","type":"boolean","isNullable":false,"hasDefault":true},{"name":"isin","type":"varchar(12)","isNullable":true,"hasDefault":false},{"name":"sector","type":"varchar(80)","isNullable":true,"hasDefault":false},{"name":"industry","type":"varchar(80)","isNullable":true,"hasDefault":false}],"key":["symbol"],"references":[{"source":{"key":["market"]},"destination":{"key":["cname"],"table":"\"market\""}}],"path":"\"public\".\"listing\""}},"domains":[],"enums":[{"type":"enum","name":"asset_type","values":["IDX","STK","FX","CFX"]}]}}


export class MarketTable {
    constructor(
        private client: IClient,
    ) {}
        
    async list(options: Partial<ListOptions<schema.Public.MarketColumn>> = {}, values: kObject = {}): Promise<Partial<schema.Public.Market>[]> { return lib.generic.list(this.client, "public", "market", options, values, _info) as Promise<Partial<schema.Public.Market>[]> }
    async create(values: schema.Public.MarketData): Promise<schema.Public.MarketKey> { return lib.generic.createUsingDefaultKey(this.client, "public", "market", {cname: null}, values) as unknown as Promise<schema.Public.MarketKey> }
    async read(key: schema.Public.MarketKey, options: Partial<ReadOptions<schema.Public.MarketColumn>> = {}): Promise<Required<schema.Public.Market>> { return lib.generic.readByKey(this.client, "public", "market", key, options, _info) as Promise<Required<schema.Public.Market>> }
    async update(key: schema.Public.MarketKey, values: Partial<schema.Public.MarketData>, options: Options = {}): Promise<void> { return lib.generic.updateByKey(this.client, "public", "market", key, values, options) }
    async delete(key: schema.Public.MarketKey): Promise<void> { return lib.generic.deleteByKey(this.client, "public", "market", key) }
    async deleteAll(filter: Clause<schema.Public.MarketColumn>, values: kObject = {}): Promise<void> { return lib.generic.deleteByFilter(this.client, "public", "market", filter, values) }
    async push(values: schema.Public.Market): Promise<void> { return lib.generic.push(this.client, "public", "market", values) }
    async pop(key: schema.Public.MarketKey): Promise<schema.Public.Market> { return lib.generic.pop(this.client, "public", "market", key) as Promise<schema.Public.Market> }
    async increment(key: schema.Public.MarketKey, values: Partial<schema.Public.MarketData>): Promise<void> { return lib.generic.incrementByKey(this.client, "public", "market", key, values) }
}


export class ProviderTable {
    constructor(
        private client: IClient,
    ) {}
        
    async list(options: Partial<ListOptions<schema.Public.ProviderColumn>> = {}, values: kObject = {}): Promise<Partial<schema.Public.Provider>[]> { return lib.generic.list(this.client, "public", "provider", options, values, _info) as Promise<Partial<schema.Public.Provider>[]> }
    async create(values: schema.Public.ProviderData): Promise<schema.Public.ProviderKey> { return lib.generic.createUsingDefaultKey(this.client, "public", "provider", {cname: null}, values) as unknown as Promise<schema.Public.ProviderKey> }
    async read(key: schema.Public.ProviderKey, options: Partial<ReadOptions<schema.Public.ProviderColumn>> = {}): Promise<Required<schema.Public.Provider>> { return lib.generic.readByKey(this.client, "public", "provider", key, options, _info) as Promise<Required<schema.Public.Provider>> }
    async update(key: schema.Public.ProviderKey, values: Partial<schema.Public.ProviderData>, options: Options = {}): Promise<void> { return lib.generic.updateByKey(this.client, "public", "provider", key, values, options) }
    async delete(key: schema.Public.ProviderKey): Promise<void> { return lib.generic.deleteByKey(this.client, "public", "provider", key) }
    async deleteAll(filter: Clause<schema.Public.ProviderColumn>, values: kObject = {}): Promise<void> { return lib.generic.deleteByFilter(this.client, "public", "provider", filter, values) }
    async push(values: schema.Public.Provider): Promise<void> { return lib.generic.push(this.client, "public", "provider", values) }
    async pop(key: schema.Public.ProviderKey): Promise<schema.Public.Provider> { return lib.generic.pop(this.client, "public", "provider", key) as Promise<schema.Public.Provider> }
    async increment(key: schema.Public.ProviderKey, values: Partial<schema.Public.ProviderData>): Promise<void> { return lib.generic.incrementByKey(this.client, "public", "provider", key, values) }
}


export class ListingTable {
    constructor(
        private client: IClient,
    ) {}
        
    async list(options: Partial<ListOptions<schema.Public.ListingColumn>> = {}, values: kObject = {}): Promise<Partial<schema.Public.Listing>[]> { return lib.generic.list(this.client, "public", "listing", options, values, _info) as Promise<Partial<schema.Public.Listing>[]> }
    async create(values: schema.Public.ListingData): Promise<schema.Public.ListingKey> { return lib.generic.createUsingDefaultKey(this.client, "public", "listing", {symbol: null}, values) as unknown as Promise<schema.Public.ListingKey> }
    async read(key: schema.Public.ListingKey, options: Partial<ReadOptions<schema.Public.ListingColumn>> = {}): Promise<Required<schema.Public.Listing>> { return lib.generic.readByKey(this.client, "public", "listing", key, options, _info) as Promise<Required<schema.Public.Listing>> }
    async update(key: schema.Public.ListingKey, values: Partial<schema.Public.ListingData>, options: Options = {}): Promise<void> { return lib.generic.updateByKey(this.client, "public", "listing", key, values, options) }
    async delete(key: schema.Public.ListingKey): Promise<void> { return lib.generic.deleteByKey(this.client, "public", "listing", key) }
    async deleteAll(filter: Clause<schema.Public.ListingColumn>, values: kObject = {}): Promise<void> { return lib.generic.deleteByFilter(this.client, "public", "listing", filter, values) }
    async push(values: schema.Public.Listing): Promise<void> { return lib.generic.push(this.client, "public", "listing", values) }
    async pop(key: schema.Public.ListingKey): Promise<schema.Public.Listing> { return lib.generic.pop(this.client, "public", "listing", key) as Promise<schema.Public.Listing> }
    async increment(key: schema.Public.ListingKey, values: Partial<schema.Public.ListingData>): Promise<void> { return lib.generic.incrementByKey(this.client, "public", "listing", key, values) }
}


class PublicSchema {
    constructor(
        private client: IClient,
    ) {}

    
    public get Market(): MarketTable {
        return new MarketTable(this.client);
    }
    

    public get Provider(): ProviderTable {
        return new ProviderTable(this.client);
    }
    

    public get Listing(): ListingTable {
        return new ListingTable(this.client);
    }
    
}

export class Api {
    constructor(
        private client: IClient,
    ) {}

    
    public get Public(): PublicSchema {
        return new PublicSchema(this.client);
    }
    
}

export const info = (): DBSchema => ({"public":{"tables":{"market":{"type":"table","name":"market","columns":[{"name":"cname","type":"varchar(8)","isNullable":false,"hasDefault":false},{"name":"name","type":"varchar(80)","isNullable":true,"hasDefault":false},{"name":"currency","type":"varchar(3)","isNullable":false,"hasDefault":true},{"name":"open","type":"time","isNullable":true,"hasDefault":false},{"name":"close","type":"time","isNullable":true,"hasDefault":false}],"key":["cname"],"references":[],"path":"\"public\".\"market\""},"provider":{"type":"table","name":"provider","columns":[{"name":"cname","type":"varchar(8)","isNullable":false,"hasDefault":false},{"name":"name","type":"varchar(80)","isNullable":true,"hasDefault":false}],"key":["cname"],"references":[],"path":"\"public\".\"provider\""},"listing":{"type":"table","name":"listing","columns":[{"name":"type","type":"asset_type","isNullable":false,"hasDefault":false},{"name":"symbol","type":"varchar(8)","isNullable":false,"hasDefault":false},{"name":"name","type":"varchar(80)","isNullable":true,"hasDefault":false},{"name":"market","type":"varchar(8)","isNullable":false,"hasDefault":false},{"name":"enabled","type":"boolean","isNullable":false,"hasDefault":true},{"name":"isin","type":"varchar(12)","isNullable":true,"hasDefault":false},{"name":"sector","type":"varchar(80)","isNullable":true,"hasDefault":false},{"name":"industry","type":"varchar(80)","isNullable":true,"hasDefault":false}],"key":["symbol"],"references":[{"source":{"key":["market"]},"destination":{"key":["cname"],"table":"\"market\""}}],"path":"\"public\".\"listing\""}},"domains":[],"enums":[{"type":"enum","name":"asset_type","values":["IDX","STK","FX","CFX"]}]}})

export default { Api, info, ...lib }
