use thiserror::Error;
use winnow::error::{ContextError, ParseError};

pub type PgsResult<T> = Result<T, PgsError>;

#[derive(Error, Debug)]
pub enum PgsError {
    #[error("Window {window_id} not found in display set {display_set}")]
    WindowNotFound { window_id: u8, display_set: String },
    #[error("Object {object_id} not found in display set {display_set}")]
    ObjectNotFound { object_id: u16, display_set: String },
    #[error("Palette {palette_id} not found in display set {display_set}")]
    PaletteNotFound {
        palette_id: u8,
        entry_id: u8,
        display_set: String,
    },
    #[error("YUV error: {0}")]
    YuvError(#[from] yuv::YuvError),
    #[error("Failed to parse PGS data: {0}")]
    ParseError(String),
}

impl<'a> From<ParseError<&'a [u8], ContextError>> for PgsError {
    fn from(e: ParseError<&'a [u8], ContextError>) -> Self {
        PgsError::ParseError(e.to_string())
    }
}
