/// Commandes Tauri — CRUD des machines virtuelles
use tauri::State;
use vm_core::{Vm, VmInput};

use super::{err, CmdResult};
use crate::state::AppState;

#[tauri::command]
pub fn create_vm(state: State<AppState>, input: VmInput) -> CmdResult<Vm> {
    let vm = state.store()?.create_vm(input).map_err(err)?;
    log::info!("VM créée : {}", vm.name);
    Ok(vm)
}

#[tauri::command]
pub fn update_vm(state: State<AppState>, id: String, input: VmInput) -> CmdResult<Vm> {
    state.store()?.update_vm(&id, input).map_err(err)
}

#[tauri::command]
pub fn delete_vm(state: State<AppState>, id: String) -> CmdResult<()> {
    state.store()?.delete_vm(&id).map_err(err)
}

#[tauri::command]
pub fn duplicate_vm(state: State<AppState>, id: String) -> CmdResult<Vm> {
    state.store()?.duplicate_vm(&id).map_err(err)
}
