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

impl From<audio_sessions::AudioError> for AppError {
    fn from(error: audio_sessions::AudioError) -> Self {
        match error {
            audio_sessions::AudioError::SessionGone => Self::SessionGone,
            audio_sessions::AudioError::AppUnknown => Self::AppUnknown,
            audio_sessions::AudioError::NotControllable => Self::NotControllable,
            audio_sessions::AudioError::InvalidVolume => Self::Validation(error.to_string()),
            audio_sessions::AudioError::Unavailable(message) => Self::AudioUnavailable(message),
            audio_sessions::AudioError::Unsupported(message) => Self::Unsupported(message),
        }
    }
}

impl From<novamixer_application::ApplicationError> for AppError {
    fn from(error: novamixer_application::ApplicationError) -> Self {
        match error {
            novamixer_application::ApplicationError::Audio(error) => error.into(),
            novamixer_application::ApplicationError::Io(error) => Self::Io(error),
            novamixer_application::ApplicationError::AppNotFound => Self::AppUnknown,
            novamixer_application::ApplicationError::GroupNotFound
            | novamixer_application::ApplicationError::SceneNotFound
            | novamixer_application::ApplicationError::LastGroup => {
                Self::Validation(error.to_string())
            }
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_the_ipc_error_contract() {
        assert_eq!(
            serde_json::to_value(AppError::AppUnknown).unwrap(),
            serde_json::json!({
                "kind": "app_unknown",
                "message": "application not found"
            })
        );
    }

    #[test]
    fn preserves_application_error_classification() {
        assert!(matches!(
            AppError::from(novamixer_application::ApplicationError::Audio(
                audio_sessions::AudioError::SessionGone
            )),
            AppError::SessionGone
        ));
        assert!(matches!(
            AppError::from(novamixer_application::ApplicationError::AppNotFound),
            AppError::AppUnknown
        ));
        assert!(matches!(
            AppError::from(novamixer_application::ApplicationError::GroupNotFound),
            AppError::Validation(_)
        ));
    }
}
