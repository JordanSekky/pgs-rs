use thiserror::Error;

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
}
