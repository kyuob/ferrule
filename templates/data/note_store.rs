use crate::domain::{error::AppError, note::Note};

pub trait NoteStore: Send {
    fn list(&self) -> Result<Vec<Note>, AppError>;
    fn save(
        &mut self,
        id: Option<i64>,
        title: &str,
        body: &str,
        modified_at_ms: i64,
    ) -> Result<Note, AppError>;
    fn delete(&mut self, id: i64) -> Result<(), AppError>;
}

pub trait Clock: Send {
    fn now_ms(&self) -> Result<i64, AppError>;
}
