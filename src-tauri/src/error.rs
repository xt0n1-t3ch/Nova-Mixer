use serde::ser::SerializeStruct;
use serde::Serialize;

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Windows audio is unavailable: {0}")]
    AudioUnavailable(String),
    #[error("the audio session no longer exists")]
    SessionGone,
    #[error("the audio session cannot be controlled")]
    NotControllable,
    #[error("application not found")]
    AppUnknown,
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let kind = match self {
            Self::AudioUnavailable(_) => "audio_unavailable",
            Self::SessionGone => "session_gone",
            Self::NotControllable => "not_controllable",
            Self::AppUnknown => "app_unknown",
            Self::Unsupported(_) => "unsupported",
            Self::Validation(_) => "validation",
            Self::Io(_) => "io",
            Self::Serde(_) => "other",
            Self::Other(_) => "other",
        };
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("kind", kind)?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
