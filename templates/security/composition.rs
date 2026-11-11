use crate::{infra::crypto::RustCrypto, usecase::private_note::PrivateNoteService};

pub fn private_notes() -> PrivateNoteService<RustCrypto> {
    PrivateNoteService(RustCrypto)
}
