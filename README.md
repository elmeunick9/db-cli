# What is DB-CLI (WIP)

DB-Cli is Not an Object Relational Mapper, written in TypeScript and designed for managing and interfacing with SQL databases (PostgreSQL).

Its main goal is not to map SQL to objects or the other way around, instead it focuses on providing functionality that is ORM adjacent such as schema definition or migrations.

It doesn't make your code DB agnostic, when using DB-CLI you write your DB in SQL, and the specific dialect you use does matter. This means you can use all features of your DB engine out of the box, but also that migrating to a different engine may not be trivial.

# Features

(Most of DB-CLI is still a WIP, this list is a wishlist for now).

* Specify schema using SQL in a well defined folder structure (see `sql` folder).
* Migrate schema to newer versions with a declarative SQL syntax.
* Generate types from your DB, including a whole data layer if needed.
* Validate SQL code.
* Manage the DB lifecycle.
* Test the DB.
* Create backups.

What DB-CLI is not:

* An ORM or a query builder, if anything is a data layer generator.
* A BaaS or a Server. There is no cloud hosting solution, this is first and foremost a library.
* An auth provider. Authentication is completely outside the scope.

# Commands

```
db init [version]
```

When in production mode, it creates the DB and fails if it already exists.

In development mode it creates/re-creates the DB and initializes it with the default data. 

Optionally you can specify a version number or alias. By default the version "next" is used in development, and the version alias "latest" is used for production. 

```
db migrate [version] [...options]
```

Migrates the DB from the current version to the specified version. As with init, by default that is "next" for development mode and "latest" for production.

If the specified version precedes the current one, a backwards migration will be performed.

If no migration path is available, it fails.

```
db release [version]
```

Creates a new release version. This involves renaming "next" to the new version and creating a new clean "next" folder.

Additionally it copies the content of the file `sql/preamble.sql` to the `YYYYMMDD.sql` file that will be automatically created in the new `next` folder.

```
db check [version]
```

```
db generate [version]
```

## Docker container for Postgres

```
docker pull postgres:15
docker run --name your-db-name -p 5432:5432 -e POSTGRES_PASSWORD=0000 -d postgres:15
```


# Migration strategy

A migration allows you to move from version A to version B of a DB without losing data. You can tell the system that a migration exists by creating a file `A.sql` in B's folder. Typically migrations are created from `latest` to `next` but they can also skip versions or even go backwards. E.g: Create an empty `YYYYMMDD.sql` file in `sql/next/` directory.

Each migration step generally works as follows:

1. Rename all schemas with a prefix "o_", e.g: "o_public".
2. Create and initialize all the schemas for the new version.
3. Copy data from e.g. "o_public" to "public", doing nescesary conversions in the process.
4. Delete the old schemas.

Step 3 will start by executing the `A.sql` script. For tables untouched by the script or the `insert.sql` files. The copy will be automated using `INSERT INTO "B"."T" SELECT ... FROM "A"."T"`. It will match by column name.

This means that data from tables affected by `insert.sql` scripts or by the migration script will not by copied over automatically. Copying can be automated in the following cases:

* No changes between the two tables.
* Adding/removing/reordering columns.
* Changing the type of a column to a compatible subtype.
* Adding/removing indices.
* Adding a column at the end with null or default.
* Adding/Deleting a table.

This means that copying needs to be manually specified in the following cases:

* Renaming a table.
* Renaming a column.
* Adding a column with not null and no default.

If there is an error during a migration step that step will be rolled back by deleting the new schemas and renaming the old ones.

In any case, migrations should be tested during development! Also, it is not a bad idea to perform a backup before each release in your pipeline.