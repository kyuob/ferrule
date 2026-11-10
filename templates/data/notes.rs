use crate::{
    domain::{error::AppError, note::Note},
    ports::note_store::{Clock, NoteStore},
};

pub struct NotesService<S: NoteStore, C: Clock> {
    store: S,
    clock: C,
}

impl<S: NoteStore, C: Clock> NotesService<S, C> {
    pub fn new(store: S, clock: C) -> Self {
        Self { store, clock }
    }
    pub fn list(&self) -> Result<Vec<Note>, AppError> {
        self.store.list()
    }
    pub fn save(&mut self, id: Option<i64>, title: String, body: String) -> Result<Note, AppError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError::InvalidInput("Enter a note title."));
        }
        if title.chars().count() > 200 {
            return Err(AppError::InvalidInput(
                "The title must be 200 characters or fewer.",
            ));
        }
        if body.chars().count() > 100_000 {
            return Err(AppError::InvalidInput("The note is too long."));
        }
        if id.is_some_and(|id| id <= 0) {
            return Err(AppError::InvalidInput("The note identifier is invalid."));
        }
        self.store.save(id, title, &body, self.clock.now_ms()?)
    }
    pub fn delete(&mut self, id: i64) -> Result<(), AppError> {
        if id <= 0 {
            return Err(AppError::InvalidInput("The note identifier is invalid."));
        }
        self.store.delete(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    #[derive(Default)]
    struct FakeStore(BTreeMap<i64, Note>);
    impl NoteStore for FakeStore {
        fn list(&self) -> Result<Vec<Note>, AppError> {
            Ok(self.0.values().cloned().collect())
        }
        fn save(
            &mut self,
            id: Option<i64>,
            title: &str,
            body: &str,
            modified_at_ms: i64,
        ) -> Result<Note, AppError> {
            if id.is_some_and(|id| !self.0.contains_key(&id)) {
                return Err(AppError::NotFound);
            }
            let id = id.unwrap_or(self.0.len() as i64 + 1);
            let note = Note {
                id,
                title: title.into(),
                body: body.into(),
                modified_at_ms,
            };
            self.0.insert(id, note.clone());
            Ok(note)
        }
        fn delete(&mut self, id: i64) -> Result<(), AppError> {
            self.0.remove(&id).map(|_| ()).ok_or(AppError::NotFound)
        }
    }
    struct FakeClock;
    impl Clock for FakeClock {
        fn now_ms(&self) -> Result<i64, AppError> {
            Ok(1234)
        }
    }
    #[test]
    fn usecases_work_through_fake_ports_without_sqlite() {
        let mut service = NotesService::new(FakeStore::default(), FakeClock);
        let note = service
            .save(None, "  메모  ".into(), "내용".into())
            .unwrap();
        assert_eq!(note.title, "메모");
        assert_eq!(note.modified_at_ms, 1234);
        assert_eq!(service.list().unwrap(), [note.clone()]);
        let updated = service
            .save(Some(note.id), "수정".into(), "새 내용".into())
            .unwrap();
        assert_eq!(updated.id, note.id);
        service.delete(note.id).unwrap();
        assert!(service.list().unwrap().is_empty());
    }
    #[test]
    fn validation_never_calls_storage_for_invalid_values() {
        let mut service = NotesService::new(FakeStore::default(), FakeClock);
        assert!(service.save(None, "  ".into(), "body".into()).is_err());
        assert!(service.save(None, "가".repeat(201), "body".into()).is_err());
        assert!(service
            .save(Some(0), "title".into(), "body".into())
            .is_err());
        assert!(service
            .save(None, "title".into(), "x".repeat(100_001))
            .is_err());
        assert!(service.list().unwrap().is_empty());
    }
}
