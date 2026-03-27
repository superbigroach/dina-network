use tracing::{debug, info};

use crate::db::{StorageError, StorageResult};
use crate::tables::STATE_METADATA;
use crate::DinaDB;

/// The current schema version of the database.
/// Increment this whenever you add a new migration step.
const CURRENT_VERSION: u32 = 1;

/// Return the current schema version constant.
pub fn current_version() -> u32 {
    CURRENT_VERSION
}

/// Read the stored schema version from the database, or 0 if none is set.
fn read_version(db: &DinaDB) -> StorageResult<u32> {
    let read_txn = db
        .inner()
        .begin_read()
        .map_err(StorageError::Transaction)?;

    let table = match read_txn.open_table(STATE_METADATA) {
        Ok(t) => t,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(0),
        Err(e) => return Err(StorageError::Table(e)),
    };

    match table.get("schema_version") {
        Ok(Some(value)) => {
            let bytes = value.value();
            if bytes.len() == 4 {
                Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
            } else {
                Ok(0)
            }
        }
        Ok(None) => Ok(0),
        Err(e) => Err(StorageError::Storage(e)),
    }
}

/// Write the schema version to the database within a write transaction.
fn write_version(db: &DinaDB, version: u32) -> StorageResult<()> {
    let write_txn = db
        .inner()
        .begin_write()
        .map_err(StorageError::Transaction)?;
    {
        let mut table = write_txn
            .open_table(STATE_METADATA)
            .map_err(StorageError::Table)?;
        let bytes = version.to_le_bytes();
        table
            .insert("schema_version", bytes.as_slice())
            .map_err(StorageError::Storage)?;
    }
    write_txn.commit().map_err(StorageError::Commit)?;
    Ok(())
}

/// Apply any pending migrations to bring the database up to `CURRENT_VERSION`.
///
/// Each migration step is idempotent: re-running `migrate` on an already
/// up-to-date database is a no-op.
pub fn migrate(db: &DinaDB) -> StorageResult<()> {
    let stored = read_version(db)?;

    if stored >= CURRENT_VERSION {
        debug!("Database schema is up to date (version {stored})");
        return Ok(());
    }

    info!(
        "Migrating database from version {stored} to {CURRENT_VERSION}"
    );

    // Migration v0 -> v1: initial table creation.
    // redb creates tables on first access, so we just need to ensure
    // all tables exist by opening them once in a write transaction.
    if stored < 1 {
        apply_v1(db)?;
    }

    // Future migrations would go here:
    // if stored < 2 { apply_v2(db)?; }

    write_version(db, CURRENT_VERSION)?;
    info!("Migration complete — now at version {CURRENT_VERSION}");
    Ok(())
}

/// v1 migration: ensure all tables are created.
fn apply_v1(db: &DinaDB) -> StorageResult<()> {
    use crate::tables::*;

    let write_txn = db
        .inner()
        .begin_write()
        .map_err(StorageError::Transaction)?;
    {
        // Opening each table in a write transaction creates it if it doesn't exist.
        write_txn.open_table(ACCOUNTS).map_err(StorageError::Table)?;
        write_txn.open_table(BLOCKS).map_err(StorageError::Table)?;
        write_txn
            .open_table(BLOCK_HASHES)
            .map_err(StorageError::Table)?;
        write_txn
            .open_table(TRANSACTIONS)
            .map_err(StorageError::Table)?;
        write_txn
            .open_table(CONTRACT_CODE)
            .map_err(StorageError::Table)?;
        write_txn
            .open_table(CONTRACT_STORAGE)
            .map_err(StorageError::Table)?;
        write_txn
            .open_table(DEVICE_REGISTRY)
            .map_err(StorageError::Table)?;
        write_txn
            .open_table(STATE_METADATA)
            .map_err(StorageError::Table)?;
    }
    write_txn.commit().map_err(StorageError::Commit)?;
    debug!("Applied migration v1: created all tables");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_version_is_positive() {
        assert!(current_version() >= 1);
    }

    #[test]
    fn migrate_idempotent() {
        let db = DinaDB::open_in_memory().expect("failed to open db");
        // migrate() was already called by open_in_memory, call it again.
        migrate(&db).expect("second migration should succeed");

        let version = read_version(&db).unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn version_persists() {
        let db = DinaDB::open_in_memory().expect("failed to open db");
        let version = read_version(&db).unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }
}
