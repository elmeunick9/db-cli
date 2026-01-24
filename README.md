# What is DB-CLI

It's a very opinionated tool written in Rust for managing SQL databases (PostgreSQL) as infrastructure-as-code.

Its main goal is to provide a solid workflow and structure for everything related to creating, migrating, testing, versioning, code generation (prepare) and deploying your DB.

# Features

* Specify schema using SQL in a well defined folder structure (see `sql` folder).
* Use LLM to create up/down migration scripts.
* Validate SQL code.
* Manage the DB lifecycle.
* Test the DB.

# Commands

## Initialization
```
db init [version]
```

When in production mode, it creates the DB and fails if it already exists.

In development mode it creates/re-creates the DB and initializes it with the default data from _`default.sql`_ and _`insert.sql`_.

Optionally you can specify a version number or alias. By default the version "next" is used in development, and the version alias "latest" is used for production.

Before initialization schemas and users may be created if they don't exist. In particular we expect the user configured as `sa` to already exist (will be the owner of the DB) and a user configured as `api` will be created if it doesn't exist with only usage permission (can not modify the schema). For most applications connecting to the DB we recommend using the `api` user.

During the initialization process files are executed in the order defined in the layers below, within a layer order is enforced using special directives "@requires/@block/@endblock" in the source code. See the provided example.

- Layer 0: `/db.sql`, for extensions/users and other DB wide setup.
- Layer 1: `**/schema.sql`, `**/*.table.sql`, `**/*.function.sql`, `**/*.view.sql`.
- Layer 2: `**/*.trigger.sql`, `**/*.index.sql`.
- Layer 3: `**/default.sql`,  `**/*.default.sql`.
- Layer 4: `**/insert.sql`,  `**/*.insert.sql`.

How to specify dependencies example:

```SQL
-- This file is: /public/schema.sql
-- @requires /public/user.table.sql

/* All code in this file will run after /public/user.table.sql */
CREATE TABLE "example1" (
    "id"        uuid        ,
    "user"      uuid        ,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("user") REFERENCES "user" ("id")
);

-- @block product
-- @requires business.company
-- @requires storage

/* This block depends on blocks "company" and "storage" */
/* The block company is in a different schema */
/* The block storage is in some unspecified file in our schema */
/* This block also depends on all previous code in this file */
CREATE TABLE "example2" (
    "id"            uuid            ,
    "company"       uuid            ,
    "storage"       uuid            ,
    "user"          uuid            ,
    "example1"      uuid            ,
    PRIMARY KEY ("id"),
    FOREIGN KEY ("company")  REFERENCES "business"."company" ("id"),
    FOREIGN KEY ("storage")  REFERENCES "storage" ("id"),
    FOREIGN KEY ("user")     REFERENCES "user" ("id"),
    FOREIGN KEY ("example1") REFERENCES "example1" ("id")
);
-- @endblock
```

## Migration

```
db plan [--from='latest'] [--to='next']
db migration --plan [--from='latest'] [--to='next']
```

> [!IMPORTANT]
> AI mode must be enabled and configured for this feature to work. Otherwise migration plans must be created manually.

> [!IMPORTANT]
> This feature is disabled in production.

Creates a migration plan (`<version>.sql`) for the DB from the specified versions. If the `to` version precedes the `from` version, a backwards migration plan will be created.

> [!WARNING]  
> Please do review and test the created migration plan before creating a release, AI may make mistakes!

```
db migration --apply [version]
```

Applies all the migration plans needed to move from the current version to the version specified. Finds the shortest path.

> [!IMPORTANT]
> To track the current version a `"public"."meta"` table (configurable) must exist satisfying or extending the following definition:
> ```SQL
> CREATE TABLE {{references.meta}} (
>   "key"   varchar(80) NOT NULL,
>   "value" text        ,
>   PRIMARY KEY ("key")
> );
> ``` 

## Version management

Every version available lives in your `sql_base` directory as a subfolder, e.g. `/sql/20260101/`. We use the YYYYMMDD format to automatically assign version numbers. Additionally the version *next* contains the files for the next planned release, that is, it's a development only version. The alias *latest* can be used in commands and points to the latest released (numeric) version.

```
db release
```

Creates a new release. This involves renaming "next" folder to the new version number and creating a new clean "next" folder (a copy with clean migration files).

If the config `releases.keep_max` is used and the new release would exceed that amount the oldest version will be deleted.

## Testing

```
db test [version]
```

This command is used to run SQL tests in the DB. This is specially useful for making unit tests of functions and stored procedures.

Initializes a new testing database `testdb_<your_db_name>` at the provided version (next by default), executes all ``**.*.test.sql` files and runs the tests.

These files must define a function that takes no parameters and returns a string in the format `[<shortid>] actual=<value> expected=<value>`. 

> [!NOTE]
> The _shortid_ is used as anchor and therefore it must be a hardcoded unique string. We recommend creating a custom command in your IDE.

Once all tests are loaded, the test runner will run them like normal functions and return an error if the returned string is not empty or `'OK'`.

# Setting up your DB

db-cli doesn't create or manage your DB server, instead it expects access to an existing instance. For development it's recommended to setup an instance using Docker (or podman). E.g:

```
docker pull postgres:18
docker run --name your-db-name -p 5432:5432 -e POSTGRES_PASSWORD=postgres -d postgres:18
```

# Configuration

Configuration is managed through a `db.toml` file in your project root. This section documents advanced configuration features.

## References

References allow you to configure database object identifiers (tables, views, functions, etc.) that may be required by third party tools or libraries.

Reference format:
```toml
[references]
meta = ["public", "meta"]
users_id = ["public", "users", ["id"]]
```

In SQL files, use references via:
- `{{references.<name>}}` - Full format
- `{{ref.<name>}}` - Short format (alias)

The references will be automatically formatted according to your `sql_dialect` setting.

## Secrets

Secrets are sensitive values (API keys, passwords, tokens, etc.) that should not be stored in version control. Unlike references, secrets are always injected as plain strings without any SQL dialect transformation.

In SQL files, use secrets via:
- `{{secrets.<name>}}`

> [!NOTE]  
> By default, secrets are NOT logged to protect sensitive data. To enable secret logging (for debugging), set `log_secrets = true` in your `db.toml`. When disabled, logged SQL will show `REDACTED` in place of secret values.

## Database Configuration Aliases

For convenience, database configuration can be referenced using both full and short format:
- `{{database.host}}` / `{{db.host}}`
- `{{database.port}}` / `{{db.port}}`
- `{{database.name}}` / `{{db.name}}`
- `{{database.sa.user}}` / `{{db.sa.user}}`
- `{{database.sa.password}}` / `{{db.sa.password}}`
- `{{database.api.user}}` / `{{db.api.user}}`
- `{{database.api.password}}` / `{{db.api.password}}`
- `{{database.ssl}}` / `{{db.ssl}}`
