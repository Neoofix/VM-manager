/// Commandes Tauri — lecture globale, export et import de l'inventaire
use std::path::PathBuf;

use serde::Serialize;
use tauri::State;
use vm_core::{Group, Vm};

use super::{err, CmdResult};
use crate::state::AppState;

#[derive(Serialize)]
pub struct Inventory {
    pub groups: Vec<Group>,
    pub vms: Vec<Vm>,
    pub data_path: String,
    /// Renseigné si le fichier de données était illisible au démarrage
    pub recovered_backup: Option<String>,
}

#[derive(Serialize)]
pub struct ImportReport {
    pub groups: usize,
    pub vms: usize,
}

#[tauri::command]
pub fn get_inventory(state: State<AppState>) -> CmdResult<Inventory> {
    let store = state.store()?;
    Ok(Inventory {
        groups: store.groups().to_vec(),
        vms: store.vms().to_vec(),
        data_path: store.path().display().to_string(),
        recovered_backup: store.recovered_backup().map(|p| p.display().to_string()),
    })
}

#[tauri::command]
pub fn export_inventory(state: State<AppState>, path: PathBuf) -> CmdResult<()> {
    state.store()?.export_to(&path).map_err(err)?;
    log::info!("Inventaire exporté vers {path:?}");
    Ok(())
}

#[tauri::command]
pub fn import_inventory(
    state: State<AppState>,
    path: PathBuf,
    replace: bool,
) -> CmdResult<ImportReport> {
    let (groups, vms) = state.store()?.import_from(&path, replace).map_err(err)?;
    log::info!("Import depuis {path:?} : {groups} groupes, {vms} VM");
    Ok(ImportReport { groups, vms })
}
