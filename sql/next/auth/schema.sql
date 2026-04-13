-- Schema based on the pg-adapter module of auth.js
-- The schema supports session storage, but we may not use it!

CREATE TABLE "verification_token" (
    "identifier"        text                NOT NULL,
    "expires"           timestamp           NOT NULL,
    "token"             text                NOT NULL,
    PRIMARY KEY ("identifier", "token")
);

-- @block user
CREATE TYPE user_role AS ENUM ('admin', 'user');
CREATE DOMAIN url AS varchar(255);
CREATE TABLE "user" (
    "id"                uuid                NOT NULL DEFAULT gen_random_uuid(),
    "created_at"        timestamp           NOT NULL DEFAULT now(),
    "name"              varchar(80)         ,
    "email"             varchar(255)        NOT NULL,
    "email_verified"    timestamp           ,
    "image"             url                 ,
    "role"              user_role           NOT NULL DEFAULT 'user',
    "ban"               uuid                ,
    "birthday"          timestamp           ,
    PRIMARY KEY ("id"),
    UNIQUE ("email")
);
-- @endblock

CREATE TABLE "account" (
    "id"                varchar(255)        NOT NULL,
    "provider"          varchar(255)        NOT NULL,
    "user"              uuid                NOT NULL,
    "type"              varchar(255)        NOT NULL,
    "refresh_token"     text                ,
    "access_token"      text                ,
    "expires_at"        bigint              ,
    "id_token"          text                ,
    "scope"             text                ,
    "session_state"     text                ,
    "token_type"        text                ,
    PRIMARY KEY ("id", "provider"),
    FOREIGN KEY ("user") REFERENCES "user" ("id") ON DELETE CASCADE
);

CREATE TABLE "session" (
    "user"              uuid                NOT NULL,
    "expires"           timestamp           NOT NULL,
    "session_token"     varchar(255)        NOT NULL,
    PRIMARY KEY ("session_token"),
    FOREIGN KEY ("user") REFERENCES "user" ("id") ON DELETE CASCADE
);

CREATE TABLE "ban" (
    "id"                uuid                NOT NULL DEFAULT gen_random_uuid(),
    "created_at"        timestamp           NOT NULL DEFAULT now(),
    "user"              uuid                NOT NULL,
    "moderator"         uuid                NOT NULL,
    "origin"            uuid                ,
    "note"              varchar(2000)       NOT NULL,
    "can_login"         boolean             NOT NULL DEFAULT true,
    "can_comment"       boolean             NOT NULL DEFAULT true,
    "can_publish"       boolean             NOT NULL DEFAULT true,
    "_expire_at"         timestamp           NOT NULL,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("user") REFERENCES "user" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("moderator") REFERENCES "user" ("id") ON DELETE CASCADE
);
ALTER TABLE "user" ADD FOREIGN KEY ("ban") REFERENCES "ban" ("id") ON DELETE SET NULL;