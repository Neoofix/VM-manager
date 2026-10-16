/// Commandes Tauri — fines enveloppes autour de `vm_core::Store`.
/// Les erreurs sont renvoyées au frontend sous forme de message lisible.
pub mod groups;
pub mod inventory;
pub mod users;
pub mod vms;

pub type CmdResult<T> = Result<T, String>;

pub fn err(e: vm_core::CoreError) -> String {
    e.to_string()
}
