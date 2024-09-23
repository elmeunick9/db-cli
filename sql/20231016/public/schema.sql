CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE "market" (
    "cname"             varchar(8)          ,
    "name"              varchar(80)         NULL,
    "currency"          varchar(3)          NOT NULL DEFAULT 'USD',
    "open"              time                NULL,
    "close"             time                NULL,
    PRIMARY KEY ("cname")
);

CREATE TABLE "provider" (
    "cname"             varchar(8)          ,
    "name"              varchar(80)         NULL,
    PRIMARY KEY ("cname")
);

CREATE TYPE asset_type AS ENUM ('IDX', 'STK', 'FX', 'CFX');
CREATE TABLE "listing" (
    "type"              asset_type          NOT NULL,
    "symbol"            varchar(8)          ,
    "name"              varchar(80)         ,
    "market"            varchar(8)          NOT NULL,
    "enabled"           boolean             NOT NULL DEFAULT true,
    "isin"              varchar(12)         ,
    "sector"            varchar(80)         ,
    "industry"          varchar(80)         ,
    PRIMARY KEY ("symbol"),
    FOREIGN KEY ("market") REFERENCES "market" ("cname"),
    UNIQUE ("type", "symbol")
);
