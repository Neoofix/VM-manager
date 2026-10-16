// Pas de console supplémentaire sous Windows en release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod forms;
mod paths;
mod prefs;
mod theme;
mod ui;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (data_path, source) = paths::resolve();
    log::info!("Fichier de données : {data_path:?} ({})", source.label());
    let store = vm_core::Store::open(&data_path).map_err(|e| e.to_string());

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("VM Manager")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([960.0, 600.0]);
    match eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(e) => log::warn!("Icône illisible : {e}"),
    }

    eframe::run_native(
        "VM Manager",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |cc| {
            let prefs_path = prefs::path_for(&data_path);
            let prefs = prefs::load(&prefs_path);
            theme::select(&cc.egui_ctx, theme::index_of(&prefs.theme));
            Ok(Box::new(app::VmManagerApp::new(
                store,
                data_path,
                source,
                prefs_path,
                cc.egui_ctx.clone(),
            )))
        }),
    )
}
