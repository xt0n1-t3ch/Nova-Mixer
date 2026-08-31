use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const PORTABLE_MARKER: &str = "portable.flag";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve(handle: &AppHandle) -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let executable_dir = executable
            .parent()
            .ok_or_else(|| "executable has no parent directory".to_string())?;
        let data_dir = if executable_dir.join(PORTABLE_MARKER).exists() {
            executable_dir.join("data")
        } else {
            handle
                .path()
                .app_data_dir()
                .map_err(|error| format!("appDataDir: {error}"))?
        };
        let config_dir = handle
            .path()
            .app_config_dir()
            .map_err(|error| format!("appConfigDir: {error}"))?;
        Ok(Self {
            logs_dir: data_dir.join("logs"),
            data_dir,
            config_dir,
        })
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        for directory in [&self.data_dir, &self.config_dir, &self.logs_dir] {
            std::fs::create_dir_all(directory)?;
        }
        Ok(())
    }
}

pub fn default_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|directory| directory.join("NovaMixer"))
}
