//! Intègre l'icône et les informations de version dans l'exécutable Windows

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico")
        .set("ProductName", "VM Manager")
        .set(
            "FileDescription",
            "VM Manager - inventaire de machines virtuelles",
        );
    if let Err(e) = res.compile() {
        // Une icône manquante ne doit pas empêcher la compilation
        println!("cargo:warning=icône Windows non intégrée : {e}");
    }
}
