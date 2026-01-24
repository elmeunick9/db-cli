CREATE DOMAIN uint4 AS int4 CHECK(VALUE >= 0 AND VALUE < 2147483648);
CREATE DOMAIN url AS varchar(255);

-- @block story
-- @requires auth.user
CREATE TABLE "story_genre" (
    "id"                uuid                NOT NULL,
    "name"              varchar(80)         NOT NULL,
    "description"       text                NOT NULL DEFAULT '',
    PRIMARY KEY ("id"),
    UNIQUE ("name")
);

CREATE TABLE "story" (
    "id"                uuid                NOT NULL DEFAULT gen_random_uuid(),
    "created_at"        timestamp           NOT NULL DEFAULT now(),
    "updated_at"        timestamp           ,
    "author"            uuid                NOT NULL,
    "title"             varchar(80)         NOT NULL,
    "subtitle"          text                NOT NULL DEFAULT '',
    "summary"           text                NOT NULL DEFAULT '',
    "views"             uint4               NOT NULL DEFAULT 0,
    "image"             url                 ,
    "media"             uuid                ,
    "approved"          boolean             NOT NULL DEFAULT false,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("author") REFERENCES "auth"."user" ("id")
);
-- @endblock

CREATE TYPE comment_state AS ENUM ('ready', 'deleted', 'deleted-by-mod');
CREATE TABLE "comment" (
    "id"                uuid                NOT NULL DEFAULT gen_random_uuid(),
    "created_at"        timestamp           NOT NULL DEFAULT now(),
    "updated_at"        timestamp           ,
    "parent"            uuid                ,
    "story"             uuid                NOT NULL,
    "author"            uuid                NOT NULL,
    "message"           varchar(2000)       NOT NULL,
    "state"             comment_state       NOT NULL DEFAULT 'ready',
    "rank"              int4                NOT NULL DEFAULT 0,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("parent") REFERENCES "comment" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("story") REFERENCES "story" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("author") REFERENCES "auth"."user" ("id") ON DELETE CASCADE
);

CREATE TYPE vote_direction AS ENUM ('up', 'down');
CREATE TABLE "comment_action" (
    "id"                uuid                ,
    "author"            uuid                ,
    "vote"              vote_direction      ,
    PRIMARY KEY ("id", "author"),
    FOREIGN KEY ("id") REFERENCES "comment" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("author") REFERENCES "auth"."user" ("id") ON DELETE CASCADE
);

CREATE TYPE mod_reason AS ENUM 
    ('hate','spam','terms','self-harm','impersonation','personal-info',
    'threatening','harassment','illegal','copyright','trademark','other',
    'first-publication');

CREATE TYPE mod_resolution AS ENUM ('allowed', 'blocked');

CREATE TABLE "comment_moderation" (
    "id"                uuid                NOT NULL DEFAULT gen_random_uuid(),
    "created_at"        timestamp           NOT NULL DEFAULT now(),
    "updated_at"        timestamp           ,
    "comment"           uuid                NOT NULL,
    "reporter"          uuid                NOT NULL,
    "resolved"          boolean             NOT NULL DEFAULT false,
    "resolved_by"       uuid                DEFAULT NULL,
    "original_message"  varchar(2000)       NOT NULL,
    "reason"            mod_reason          NOT NULL,
    "reason_detail"     varchar(2000)       NOT NULL DEFAULT '',
    "resolution"        mod_resolution      ,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("comment") REFERENCES "comment" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("reporter") REFERENCES "auth"."user" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("resolved_by") REFERENCES "auth"."user" ("id") ON DELETE SET NULL
);

CREATE TABLE "story_moderation" (
    "id"                uuid                NOT NULL DEFAULT gen_random_uuid(),
    "created_at"        timestamp           NOT NULL DEFAULT now(),
    "updated_at"        timestamp           ,
    "story"             uuid                NOT NULL,
    "reporter"          uuid                ,
    "resolved"          boolean             NOT NULL DEFAULT false,
    "resolved_by"       uuid                DEFAULT NULL,
    "reason"            mod_reason          NOT NULL,
    "reason_detail"     varchar(2000)       NOT NULL DEFAULT '',
    "resolution"        mod_resolution      ,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("story") REFERENCES "story" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("reporter") REFERENCES "auth"."user" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("resolved_by") REFERENCES "auth"."user" ("id") ON DELETE SET NULL
);

CREATE TYPE inv_grant AS ENUM ('preview', 'read', 'read-write', 'publish');
CREATE TABLE "invitation" (
    "id"                uuid                NOT NULL DEFAULT gen_random_uuid(),
    "created_at"        timestamp           NOT NULL DEFAULT now(),
    "expire_at"         timestamp           ,
    "story"             uuid                NOT NULL,
    "user"              uuid                ,
    "grant"             inv_grant           NOT NULL,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("story") REFERENCES "story" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("user") REFERENCES "auth"."user" ("id") ON DELETE CASCADE
);

CREATE TABLE "meta" (
    "key"               varchar(100)        NOT NULL,
    "updated_at"        timestamp           NOT NULL DEFAULT now(),
    "value"             text                NOT NULL,
    PRIMARY KEY ("key")
);