/// Commandes Tauri — utilisateurs associés à une VM.
/// Chaque commande renvoie la VM à jour pour que le frontend la remplace d'un bloc.
use tauri::State;
use vm_core::{Store, Vm, VmUserInput};

use super::{err, CmdResult};
use crate::state::AppState;

fn snapshot(store: &Store, vm_id: &str) -> CmdResult<Vm> {
    store.vm(vm_id).cloned().map_err(err)
}

#[tauri::command]
pub fn add_vm_user(state: State<AppState>, vm_id: String, input: VmUserInput) -> CmdResult<Vm> {
    let mut store = state.store()?;
    store.add_user(&vm_id, input).map_err(err)?;
    snapshot(&store, &vm_id)
}

#[tauri::command]
pub fn update_vm_user(
    state: State<AppState>,
    vm_id: String,
    user_id: String,
    input: VmUserInput,
) -> CmdResult<Vm> {
    let mut store = state.store()?;
    store.update_user(&vm_id, &user_id, input).map_err(err)?;
    snapshot(&store, &vm_id)
}

#[tauri::command]
pub fn delete_vm_user(state: State<AppState>, vm_id: String, user_id: String) -> CmdResult<Vm> {
    let mut store = state.store()?;
    store.delete_user(&vm_id, &user_id).map_err(err)?;
    snapshot(&store, &vm_id)
}
