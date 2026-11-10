use crate::{
    domain::{error::AppError, note::Note},
    infra::sqlite::SqliteNoteStore,
    ports::note_store::Clock,
    usecase::notes::NotesService,
};
use std::{
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct SystemClock;
impl Clock for SystemClock {
    fn now_ms(&self) -> Result<i64, AppError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|value| value.as_millis().try_into().ok())
            .ok_or(AppError::Unavailable)
    }
}

pub struct NotesFacade {
    service: Mutex<Option<NotesService<SqliteNoteStore, SystemClock>>>,
}
impl NotesFacade {
    pub fn open(directory: &str) -> Result<Self, AppError> {
        Ok(Self {
            service: Mutex::new(Some(NotesService::new(
                SqliteNoteStore::open(Path::new(directory))?,
                SystemClock,
            ))),
        })
    }
    pub fn list(&self) -> Result<Vec<Note>, AppError> {
        self.service
            .lock()
            .map_err(|_| AppError::Unavailable)?
            .as_ref()
            .ok_or(AppError::Unavailable)?
            .list()
    }
    pub fn save(&self, id: Option<i64>, title: String, body: String) -> Result<Note, AppError> {
        self.service
            .lock()
            .map_err(|_| AppError::Unavailable)?
            .as_mut()
            .ok_or(AppError::Unavailable)?
            .save(id, title, body)
    }
    pub fn delete(&self, id: i64) -> Result<(), AppError> {
        self.service
            .lock()
            .map_err(|_| AppError::Unavailable)?
            .as_mut()
            .ok_or(AppError::Unavailable)?
            .delete(id)
    }
    pub fn close(&self) -> Result<(), AppError> {
        self.service
            .lock()
            .map_err(|_| AppError::Unavailable)?
            .take();
        Ok(())
    }
}
