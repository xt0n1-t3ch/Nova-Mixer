use novamixer_contracts::AppSettings;
use parking_lot::RwLock;

pub struct AppState {
    pub settings: RwLock<AppSettings>,
    pub application: novamixer_application::NovaMixerApplication,
}

impl AppState {
    pub fn new(application: novamixer_application::NovaMixerApplication) -> Self {
        Self {
            settings: RwLock::new(application.settings()),
            application,
        }
    }
}
