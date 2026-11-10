use crate::{composition::data::NotesFacade, domain::note::Note};

pub struct NoteRecord {
    pub id: String,
    pub title: String,
    pub body: String,
    pub modified_at_ms: i64,
}
impl From<Note> for NoteRecord {
    fn from(note: Note) -> Self {
        Self {
            id: note.id.to_string(),
            title: note.title,
            body: note.body,
            modified_at_ms: note.modified_at_ms,
        }
    }
}

#[flutter_rust_bridge::frb(opaque)]
pub struct NotesSession {
    facade: NotesFacade,
}

/// Non-sync bridge calls run on FRB's worker pool, not Flutter's UI isolate.
pub fn open_notes(directory: String) -> Result<NotesSession, String> {
    NotesFacade::open(&directory)
        .map(|facade| NotesSession { facade })
        .map_err(|error| error.to_string())
}

fn note_id(id: &str) -> Result<i64, String> {
    id.parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| "The note identifier is invalid.".into())
}

impl NotesSession {
    pub fn list(&self) -> Result<Vec<NoteRecord>, String> {
        self.facade
            .list()
            .map(|notes| notes.into_iter().map(Into::into).collect())
            .map_err(|error| error.to_string())
    }
    pub fn save(
        &self,
        id: Option<String>,
        title: String,
        body: String,
    ) -> Result<NoteRecord, String> {
        let id = id.as_deref().map(note_id).transpose()?;
        self.facade
            .save(id, title, body)
            .map(Into::into)
            .map_err(|error| error.to_string())
    }
    pub fn delete(&self, id: String) -> Result<(), String> {
        self.facade
            .delete(note_id(&id)?)
            .map_err(|error| error.to_string())
    }
    pub fn close(&self) -> Result<(), String> {
        self.facade.close().map_err(|error| error.to_string())
    }
}
