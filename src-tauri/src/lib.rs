//! Application Tauri : expose l'inventaire de `vm-core` au frontend React
mod commands;
mod state;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::load(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::inventory::get_inventory,
            commands::inventory::export_inventory,
            commands::inventory::import_inventory,
            commands::groups::create_group,
            commands::groups::update_group,
            commands::groups::delete_group,
            commands::vms::create_vm,
            commands::vms::update_vm,
            commands::vms::delete_vm,
            commands::vms::duplicate_vm,
            commands::users::add_vm_user,
            commands::users::update_vm_user,
            commands::users::delete_vm_user,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur au lancement de l'application Tauri");
}
