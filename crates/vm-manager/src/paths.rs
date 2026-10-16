/// Emplacement du fichier de données.
///
/// Par défaut l'application est **portable** : l'inventaire est enregistré à
/// côté de l'exécutable (`vm-manager.json`). Ordre de priorité :
/// 1. argument `--data <chemin>` (ou `--data=<chemin>`) ;
/// 2. variable d'environnement `VM_MANAGER_DATA` ;
/// 3. dossier de l'exécutable, s'il est accessible en écriture ;
/// 4. sinon (application installée par le `.msi` ou le `.deb`) : dossier de
///    données de l'utilisateur — `%APPDATA%\VM Manager` sous Windows,
///    `~/.local/share/vm-manager` sous Linux, `~/Library/Application Support/VM Manager` sous macOS ;
/// 5. en dernier recours, dossier courant.
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
    UserDataDir,
    CurrentDir,
}

impl DataSource {
    pub fn label(self) -> &'static str {
        match self {
            DataSource::Argument => "argument --data",
            DataSource::Environment => "variable VM_MANAGER_DATA",
            DataSource::ExecutableDir => "dossier de l'exécutable (mode portable)",
            DataSource::UserDataDir => {
                "dossier de données de l'utilisateur (application installée)"
            }
            DataSource::CurrentDir => "dossier courant",
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
        user_data_dir(),
        |dir| !is_system_dir(dir) && is_writable_dir(dir),
    )
}

/// Dossier d'installation système (paquet .deb, installateur .msi) : jamais utilisé
/// pour les données, même si l'application tourne avec des droits administrateur
fn is_system_dir(dir: &Path) -> bool {
    if cfg!(windows) {
        ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]
            .iter()
            .filter_map(std::env::var_os)
            .any(|root| dir.starts_with(root))
    } else {
        ["/usr", "/opt", "/bin", "/sbin", "/snap", "/Applications"]
            .iter()
            .any(|root| dir.starts_with(root))
    }
}

/// Dossier de données propre à l'utilisateur, selon la plateforme
fn user_data_dir() -> Option<PathBuf> {
    let var = |name: &str| {
        std::env::var_os(name)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if cfg!(windows) {
        var("APPDATA").map(|d| d.join("VM Manager"))
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support/VM Manager"))
    } else {
        var("XDG_DATA_HOME")
            .or_else(|| var("HOME").map(|h| h.join(".local/share")))
            .map(|d| d.join("vm-manager"))
    }
}

/// Version pure de [`resolve`], testable
pub fn resolve_from(
    args: impl IntoIterator<Item = OsString>,
    env: Option<OsString>,
    exe_dir: Option<PathBuf>,
    user_dir: Option<PathBuf>,
    writable: impl Fn(&Path) -> bool,
) -> (PathBuf, DataSource) {
    if let Some(path) = data_arg(args) {
        return (path, DataSource::Argument);
    }
    if let Some(path) = env.filter(|v| !v.is_empty()) {
        return (path.into(), DataSource::Environment);
    }
    if let Some(dir) = exe_dir.filter(|d| writable(d)) {
        return (dir.join(DATA_FILE_NAME), DataSource::ExecutableDir);
    }
    match user_dir {
        // Le dossier est créé à l'ouverture de l'inventaire
        Some(dir) => (dir.join(DATA_FILE_NAME), DataSource::UserDataDir),
        None => (PathBuf::from(DATA_FILE_NAME), DataSource::CurrentDir),
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

    fn user() -> Option<PathBuf> {
        Some("/home/u/.local/share/vm-manager".into())
    }

    #[test]
    fn argument_wins() {
        let (p, s) = resolve_from(
            os(&["--data", "/x/inv.json"]),
            Some("/env.json".into()),
            Some("/opt/app".into()),
            user(),
            |_| true,
        );
        assert_eq!((p, s), (PathBuf::from("/x/inv.json"), DataSource::Argument));
        let (p, _) = resolve_from(os(&["--data=/y.json"]), None, None, None, |_| true);
        assert_eq!(p, PathBuf::from("/y.json"));
    }

    #[test]
    fn environment_then_executable_dir() {
        let (p, s) = resolve_from(
            os(&[]),
            Some("/env.json".into()),
            Some("/opt/app".into()),
            user(),
            |_| true,
        );
        assert_eq!(
            (p, s),
            (PathBuf::from("/env.json"), DataSource::Environment)
        );

        let (p, s) = resolve_from(
            os(&[]),
            Some("".into()),
            Some("/opt/app".into()),
            user(),
            |_| true,
        );
        assert_eq!(p, PathBuf::from("/opt/app").join(DATA_FILE_NAME));
        assert_eq!(s, DataSource::ExecutableDir);
    }

    #[test]
    fn installed_app_uses_user_data_dir() {
        // Exécutable dans /usr/bin ou Program Files : non modifiable
        let (p, s) = resolve_from(os(&[]), None, Some("/usr/bin".into()), user(), |_| false);
        assert_eq!(
            p,
            PathBuf::from("/home/u/.local/share/vm-manager").join(DATA_FILE_NAME)
        );
        assert_eq!(s, DataSource::UserDataDir);
    }

    #[test]
    fn falls_back_to_current_dir_last() {
        let (p, s) = resolve_from(os(&[]), None, Some("/usr/bin".into()), None, |_| false);
        assert_eq!(
            (p, s),
            (PathBuf::from(DATA_FILE_NAME), DataSource::CurrentDir)
        );
    }

    #[test]
    #[cfg(unix)]
    fn system_dirs_are_detected() {
        assert!(is_system_dir(Path::new("/usr/bin")));
        assert!(is_system_dir(Path::new("/opt/vm-manager")));
        assert!(!is_system_dir(Path::new("/home/u/outils")));
        assert!(!is_system_dir(Path::new("/usrlocal")));
    }

    #[test]
    fn writable_probe_on_real_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(is_writable_dir(dir.path()));
        assert!(!is_writable_dir(&dir.path().join("absent")));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
