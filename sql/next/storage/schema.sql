-- @requires /public/schema.sql
CREATE DOMAIN uint8 AS int8 CHECK(VALUE >= 0 AND VALUE < 9223372036854775808);
CREATE DOMAIN size_kb AS int4 CHECK(VALUE >= 0 AND VALUE < 2147483648);

-- If used > allocated some restrictions may apply.
-- Used is recalculated on heavy load actions such as creating or deleting a media.
-- Used may also be recalculated on smaller size actions depending on how old updated_at is.
-- Recalculating used or changing allocated should update the timestamp.

CREATE TABLE "storage_account" (
    "user"              uuid                ,
    "updated_at"        timestamp           ,
    "allocated"         size_kb             NOT NULL DEFAULT 50 * 1024,
    "used"              size_kb             NOT NULL DEFAULT 0,
    PRIMARY KEY ("user"),
    FOREIGN KEY ("user") REFERENCES "auth"."user" ("id") ON DELETE CASCADE
);

-- We do not do DELETE CASCADE since we need to remove the actual files from object storage first.
-- Instead, we use the "pending-deletion" state.

CREATE TYPE media_state AS ENUM ('ready', 'in-progress', 'pending-validation', 'pending-deletion', 'failed');
CREATE TABLE "media" (
    "id"                uuid                NOT NULL DEFAULT gen_random_uuid(),
    "created_at"        timestamp           NOT NULL DEFAULT now(),
    "updated_at"        timestamp           ,
    "story"             uuid                NOT NULL,
    "source"            boolean             NOT NULL DEFAULT false,
    "version"           varchar(20)         NOT NULL DEFAULT '0.0.0',
    "state"             media_state         NOT NULL,
    "size"              size_kb             NOT NULL DEFAULT 0,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("story") REFERENCES "public"."story" ("id"),
    UNIQUE ("story", "source", "version"),
    EXCLUDE ("story" WITH =) WHERE ("source")
);

-- Index of all files within a media resource. 
-- Needed for fast retrieval and modification specially in the media used by the editor.
-- Medias for releases do no need to use this since the index is static. 
-- Also used to calculate stoarge usage.

CREATE TABLE "file" (
    "media"             uuid                NOT NULL,
    "parent"            uuid                ,
    "inode"             uuid                NOT NULL DEFAULT gen_random_uuid(),
    "name"              varchar(80)         NOT NULL,
    "size"              size_kb             NOT NULL DEFAULT 0,
    "directory"         boolean             NOT NULL DEFAULT false,
    PRIMARY KEY ("media", "inode"),
    FOREIGN KEY ("media") REFERENCES "media" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("parent") REFERENCES "file" ("inode") ON DELETE CASCADE,
    UNIQUE ("media", "parent", "name"),
    UNIQUE ("inode")
);

-- Used by the editor. Contains only source code files. The idea is to offload them
-- to object storage (compressed) if it's been a long time since last used.
-- Like with file, an index is also build from this table.

CREATE TABLE "source" (
    "media"             uuid                NOT NULL,
    "parent"            uuid                ,
    "inode"             uuid                NOT NULL DEFAULT gen_random_uuid(),
    "name"              varchar(80)         NOT NULL,
    "size"              size_kb             NOT NULL DEFAULT 0,
    "timestamp"         timestamp           NOT NULL DEFAULT now(),
    "text"              text                NOT NULL DEFAULT '',
    "directory"         boolean             NOT NULL DEFAULT false,
    PRIMARY KEY ("media", "inode"),
    FOREIGN KEY ("media") REFERENCES "media" ("id") ON DELETE CASCADE,
    FOREIGN KEY ("parent") REFERENCES "source" ("inode") ON DELETE CASCADE,
    UNIQUE ("media", "parent", "name"),
    UNIQUE ("inode")
);

