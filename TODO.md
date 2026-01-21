# TODO List for Improving create_db Function

## Completed Tasks
- [x] Add `maintenance_db_name` field to `DatabaseConfig` struct in `src/config.rs` with default "postgres"
- [x] Update `Default` implementation for `Config` to include `maintenance_db_name`
- [x] Update `parse_database_url` function to include `maintenance_db_name`
- [x] Modify `create_db` in `src/utils/db.rs` to use `config.database.maintenance_db_name` instead of hardcoded "postgres"
- [x] Add `is_database_empty` helper function to check if database has no user-created objects and only the public schema
- [x] Update production logic in `create_db` to warn and skip CREATE DATABASE if database exists but is empty, error if not empty

## Verification
- [x] Code compiles successfully with `cargo check`
