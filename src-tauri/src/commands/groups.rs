/// Commandes Tauri — CRUD des groupes de VM
use tauri::State;
use vm_core::{Group, GroupInput};

use super::{err, CmdResult};
use crate::state::AppState;

#[tauri::command]
pub fn create_group(state: State<AppState>, input: GroupInput) -> CmdResult<Group> {
    let group = state.store()?.create_group(input).map_err(err)?;
    log::info!("Groupe créé : {}", group.name);
    Ok(group)
}

#[tauri::command]
pub fn update_group(state: State<AppState>, id: String, input: GroupInput) -> CmdResult<Group> {
    state.store()?.update_group(&id, input).map_err(err)
}

/// Renvoie le nombre de VM détachées du groupe supprimé
#[tauri::command]
pub fn delete_group(state: State<AppState>, id: String) -> CmdResult<usize> {
    state.store()?.delete_group(&id).map_err(err)
}
