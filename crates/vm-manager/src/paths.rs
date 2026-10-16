/// Emplacement du fichier de données.
///
/// Par défaut l'application est **portable** : l'inventaire est enregistré à
/// côté de l'exécutable (`vm-manager.json`). Ordre de priorité :
/// 1. argument `--data <chemin>` (ou `--data=<chemin>`) ;
/// 2. variable d'environnement `VM_MANAGER_DATA` ;
/// 3. dossier de l'exécutable, s'il est accessible en écriture ;
/// 4. dossier courant.
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub const DATA_FILE_NAME: &str = "vm-manager.json";
pub const DATA_ENV: &str = "VM_MANAGER_DATA";

/// Origine du chemin retenu, affichée dans les Paramètres
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    Argument,
    Environment,
    ExecutableDir,
    CurrentDir,
}

impl DataSource {
    pub fn label(self) -> &'static str {
        match self {
            DataSource::Argument => "argument --data",
            DataSource::Environment => "variable VM_MANAGER_DATA",
            DataSource::ExecutableDir => "dossier de l'exécutable (mode portable)",
            DataSource::CurrentDir => "dossier courant (dossier de l'exécutable non modifiable)",
        }
    }
}

/// Résout le chemin à partir de l'environnement réel du processus
pub fn resolve() -> (PathBuf, DataSource) {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    resolve_from(
        std::env::args_os().skip(1),
        std::env::var_os(DATA_ENV),
        exe_dir,
        is_writable_dir,
    )
}

/// Version pure de [`resolve`], testable
pub fn resolve_from(
    args: impl IntoIterator<Item = OsString>,
    env: Option<OsString>,
    exe_dir: Option<PathBuf>,
    writable: impl Fn(&Path) -> bool,
) -> (PathBuf, DataSource) {
    if let Some(path) = data_arg(args) {
        return (path, DataSource::Argument);
    }
    if let Some(path) = env.filter(|v| !v.is_empty()) {
        return (path.into(), DataSource::Environment);
    }
    match exe_dir {
        Some(dir) if writable(&dir) => (dir.join(DATA_FILE_NAME), DataSource::ExecutableDir),
        _ => (PathBuf::from(DATA_FILE_NAME), DataSource::CurrentDir),
    }
}

fn data_arg(args: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let Some(text) = arg.to_str() else { continue };
        if text == "--data" {
            return args.next().map(PathBuf::from);
        }
        if let Some(value) = text.strip_prefix("--data=") {
            return Some(PathBuf::from(value));
        }
    }
    None
}

/// Vérifie réellement l'écriture (les permissions affichées ne suffisent pas
/// sous Windows, ex. `Program Files`)
fn is_writable_dir(dir: &Path) -> bool {
    let probe = dir.join(".vm-manager-write-test");
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(v: &[&str]) -> Vec<OsString> {
        v.iter().map(OsString::from).collect()
    }

    #[test]
    fn argument_wins() {
        let (p, s) = resolve_from(
            os(&["--data", "/x/inv.json"]),
            Some("/env.json".into()),
            Some("/opt/app".into()),
            |_| true,
        );
        assert_eq!((p, s), (PathBuf::from("/x/inv.json"), DataSource::Argument));
        let (p, _) = resolve_from(os(&["--data=/y.json"]), None, None, |_| true);
        assert_eq!(p, PathBuf::from("/y.json"));
    }

    #[test]
    fn environment_then_executable_dir() {
        let (p, s) = resolve_from(
            os(&[]),
            Some("/env.json".into()),
            Some("/opt/app".into()),
            |_| true,
        );
        assert_eq!(
            (p, s),
            (PathBuf::from("/env.json"), DataSource::Environment)
        );

        let (p, s) = resolve_from(os(&[]), Some("".into()), Some("/opt/app".into()), |_| true);
        assert_eq!(p, PathBuf::from("/opt/app").join(DATA_FILE_NAME));
        assert_eq!(s, DataSource::ExecutableDir);
    }

    #[test]
    fn falls_back_to_current_dir_when_not_writable() {
        let (p, s) = resolve_from(os(&[]), None, Some("/opt/app".into()), |_| false);
        assert_eq!(
            (p, s),
            (PathBuf::from(DATA_FILE_NAME), DataSource::CurrentDir)
        );
    }

    #[test]
    fn writable_probe_on_real_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(is_writable_dir(dir.path()));
        assert!(!is_writable_dir(&dir.path().join("absent")));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
