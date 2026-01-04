# What is DB-CLI

It's a very opinionated tool written in Python for managing SQL databases (PostgreSQL) as infrastructure-as-code.

Its main goal is to provide a solid workflow and structure for everything related to creating, migrating, testing, versioning, code generation (prepare) and deploying your DB.

# Features

* Specify schema using SQL in a well defined folder structure (see `sql` folder).
* Use LLM to create up/down migration scripts.
* Validate SQL code.
* Manage the DB lifecycle.
* Test the DB.

# Commands

```
db init [version]
```

When in production mode, it creates the DB and fails if it already exists.

In development mode it creates/re-creates the DB and initializes it with the default data. 

Optionally you can specify a version number or alias. By default the version "next" is used in development, and the version alias "latest" is used for production. 

```
db plan [version]
```

Creates a migration plan (up.sql) for the DB from the current version to the specified version. As with init, by default that is "next" for development mode and "latest" for production.

If the specified version precedes the current one, a backwards migration plan will be created. (down.sql)

It uses a configured LLM to do this.

```
db migration --plan [version]
```

Creates a migration plan (up.sql) for the DB from the current version to the specified version. As with init, by default that is "next" for development mode and "latest" for production.

If the specified version precedes the current one, a backwards migration plan will be created. (down.sql)

It uses a configured LLM to do this.

```
db migration --apply [version]
```

Applies all the migration plans needed to move from the current version to the version specified.

```
db release [version]
```

Creates a new release version. This involves renaming "next" to the new version and creating a new clean "next" folder.

```
db check [version]
```

*TODO* Runs SQL tests.

```
db code-generate [version]
```

Runs a configured command that will perform code generation for your project, e.g. `sqlx prepare`.

## Setting up your DB

db-cli doesn't create or manage your DB server, instead it expects access to an existing instance. For development it's recommended to setup an instance using Docker (or podman). E.g:

```
docker pull postgres:15
docker run --name your-db-name -p 5432:5432 -e POSTGRES_PASSWORD=0000 -d postgres:15
```