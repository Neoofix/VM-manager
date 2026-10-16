/// État partagé entre les commandes : l'inventaire chargé depuis `data.json`
use std::sync::{Mutex, MutexGuard};

use tauri::{AppHandle, Manager};
use vm_core::Store;

pub struct AppState {
    store: Mutex<Store>,
}

impl AppState {
    /// Ouvre `<dossier de données de l'application>/data.json`.
    /// La variable `VM_MANAGER_DATA` permet de pointer vers un autre fichier.
    pub fn load(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let path = match std::env::var_os("VM_MANAGER_DATA") {
            Some(custom) => custom.into(),
            None => app.path().app_data_dir()?.join("data.json"),
        };
        let store = Store::open(path)?;
        Ok(AppState {
            store: Mutex::new(store),
        })
    }

    pub fn store(&self) -> Result<MutexGuard<'_, Store>, String> {
        self.store
            .lock()
            .map_err(|e| format!("Erreur de verrouillage de l'inventaire : {e}"))
    }
}
