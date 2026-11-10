use crate::{
    domain::{error::AppError, note::Note},
    ports::note_store::NoteStore,
};
use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};
use std::path::Path;

const APPLICATION_ID: i64 = 0x4652524c;
const SCHEMA_VERSION: i64 = 2;

pub struct SqliteNoteStore {
    connection: Connection,
}

impl SqliteNoteStore {
    pub fn open(directory: &Path) -> Result<Self, AppError> {
        if !directory.is_absolute() || !directory.is_dir() {
            return Err(AppError::InvalidInput(
                "Use an existing app-support directory.",
            ));
        }
        let path = directory.join("ferrule-notes.sqlite3");
        if path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(AppError::Storage);
        }
        let mut connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| AppError::Storage)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| AppError::Storage)?;
        migrate(&mut connection)?;
        Ok(Self { connection })
    }
}

fn migrate(connection: &mut Connection) -> Result<(), AppError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| AppError::Storage)?;
    let version: i64 = tx
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| AppError::Storage)?;
    let application_id: i64 = tx
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .map_err(|_| AppError::Storage)?;
    if version > SCHEMA_VERSION {
        return Err(AppError::SchemaTooNew);
    }
    if version < 0 || (application_id != APPLICATION_ID && !(version == 0 && application_id == 0)) {
        return Err(AppError::Storage);
    }
    if version == 0 {
        let tables: i64 = tx
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(|_| AppError::Storage)?;
        if tables != 0 {
            return Err(AppError::Storage);
        }
        tx.execute_batch("CREATE TABLE notes (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, body TEXT NOT NULL); PRAGMA user_version = 1;").map_err(|_| AppError::Storage)?;
    }
    if version < 2 {
        tx.execute_batch("ALTER TABLE notes ADD COLUMN modified_at_ms INTEGER NOT NULL DEFAULT 0; PRAGMA user_version = 2;").map_err(|_| AppError::Storage)?;
    }
    // Catch malformed or mismatched schemas before returning a usable store.
    let mut statement = tx
        .prepare("PRAGMA table_info(notes)")
        .map_err(|_| AppError::Storage)?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|_| AppError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::Storage)?;
    if names != ["id", "title", "body", "modified_at_ms"] {
        return Err(AppError::Storage);
    }
    drop(statement);
    tx.pragma_update(None, "application_id", APPLICATION_ID)
        .map_err(|_| AppError::Storage)?;
    tx.commit().map_err(|_| AppError::Storage)
}

impl NoteStore for SqliteNoteStore {
    fn list(&self) -> Result<Vec<Note>, AppError> {
        let mut query = self.connection.prepare("SELECT id, title, body, modified_at_ms FROM notes ORDER BY modified_at_ms DESC, id DESC").map_err(|_| AppError::Storage)?;
        let rows = query
            .query_map([], |row| {
                Ok(Note {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    body: row.get(2)?,
                    modified_at_ms: row.get(3)?,
                })
            })
            .map_err(|_| AppError::Storage)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::Storage)
    }
    fn save(
        &mut self,
        id: Option<i64>,
        title: &str,
        body: &str,
        modified_at_ms: i64,
    ) -> Result<Note, AppError> {
        let tx = self
            .connection
            .transaction()
            .map_err(|_| AppError::Storage)?;
        let id = if let Some(id) = id {
            let count = tx
                .execute(
                    "UPDATE notes SET title = ?1, body = ?2, modified_at_ms = ?3 WHERE id = ?4",
                    params![title, body, modified_at_ms, id],
                )
                .map_err(|_| AppError::Storage)?;
            if count == 0 {
                return Err(AppError::NotFound);
            }
            id
        } else {
            tx.execute(
                "INSERT INTO notes(title, body, modified_at_ms) VALUES(?1, ?2, ?3)",
                params![title, body, modified_at_ms],
            )
            .map_err(|_| AppError::Storage)?;
            tx.last_insert_rowid()
        };
        tx.commit().map_err(|_| AppError::Storage)?;
        Ok(Note {
            id,
            title: title.into(),
            body: body.into(),
            modified_at_ms,
        })
    }
    fn delete(&mut self, id: i64) -> Result<(), AppError> {
        let count = self
            .connection
            .execute("DELETE FROM notes WHERE id = ?1", [id])
            .map_err(|_| AppError::Storage)?;
        if count == 0 {
            Err(AppError::NotFound)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Keep generated app tests dependency-free beyond the selected module.
    struct TestDirectory(std::path::PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ferrule-notes-{}-{stamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn persisted_notes_survive_reopen_and_parameterized_queries() {
        let dir = TestDirectory::new();
        let mut store = SqliteNoteStore::open(&dir.0).unwrap();
        let note = store
            .save(None, "한글 ' ; DROP TABLE notes; --", "body 📝", 123)
            .unwrap();
        drop(store);
        let mut store = SqliteNoteStore::open(&dir.0).unwrap();
        assert_eq!(store.list().unwrap(), [note.clone()]);
        store.save(Some(note.id), "updated", "next", 456).unwrap();
        assert_eq!(store.list().unwrap()[0].modified_at_ms, 456);
        store.delete(note.id).unwrap();
        assert!(store.list().unwrap().is_empty());
        assert_eq!(store.delete(note.id), Err(AppError::NotFound));
    }
    #[test]
    fn migrates_v1_and_preserves_user_content() {
        let dir = TestDirectory::new();
        let connection = Connection::open(dir.0.join("ferrule-notes.sqlite3")).unwrap();
        connection.execute_batch("CREATE TABLE notes(id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, body TEXT NOT NULL); INSERT INTO notes(title,body) VALUES('old','content'); PRAGMA user_version=1;").unwrap();
        connection
            .pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        drop(connection);
        let store = SqliteNoteStore::open(&dir.0).unwrap();
        assert_eq!(store.list().unwrap()[0].body, "content");
        assert_eq!(store.list().unwrap()[0].modified_at_ms, 0);
    }
    #[test]
    fn refuses_future_or_unrelated_database_without_destroying_content() {
        for version in [0, 99] {
            let dir = TestDirectory::new();
            let connection = Connection::open(dir.0.join("ferrule-notes.sqlite3")).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE precious(value TEXT); INSERT INTO precious VALUES('keep');",
                )
                .unwrap();
            connection
                .pragma_update(None, "user_version", version)
                .unwrap();
            drop(connection);
            assert!(SqliteNoteStore::open(&dir.0).is_err());
            let connection = Connection::open(dir.0.join("ferrule-notes.sqlite3")).unwrap();
            assert_eq!(
                connection
                    .query_row("SELECT value FROM precious", [], |row| row
                        .get::<_, String>(0))
                    .unwrap(),
                "keep"
            );
        }
    }
    #[test]
    fn failed_migration_rolls_back_schema_version() {
        let dir = TestDirectory::new();
        let connection = Connection::open(dir.0.join("ferrule-notes.sqlite3")).unwrap();
        connection.execute_batch("CREATE TABLE notes(id INTEGER, title TEXT, body TEXT, modified_at_ms INTEGER); PRAGMA user_version=1;").unwrap();
        connection
            .pragma_update(None, "application_id", APPLICATION_ID)
            .unwrap();
        drop(connection);
        assert!(SqliteNoteStore::open(&dir.0).is_err());
        let connection = Connection::open(dir.0.join("ferrule-notes.sqlite3")).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
