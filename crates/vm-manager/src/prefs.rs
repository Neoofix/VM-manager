/// Préférences d'affichage (thème), enregistrées à côté du fichier de données :
/// `vm-manager.json` → `vm-manager.prefs.json`. Séparées de l'inventaire pour
/// qu'un export / import ne transporte pas les réglages d'interface.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    /// Identifiant du thème (voir `theme::THEMES`)
    #[serde(default)]
    pub theme: String,
}

pub fn path_for(data_path: &Path) -> PathBuf {
    let stem = data_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "vm-manager".into());
    data_path.with_file_name(format!("{stem}.prefs.json"))
}

/// Fichier absent ou illisible : préférences par défaut
pub fn load(path: &Path) -> Prefs {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, prefs: &Prefs) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(prefs).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_is_next_to_data_file() {
        assert_eq!(
            path_for(Path::new("/opt/app/vm-manager.json")),
            PathBuf::from("/opt/app/vm-manager.prefs.json")
        );
        assert_eq!(
            path_for(Path::new("inv.json")),
            PathBuf::from("inv.prefs.json")
        );
    }

    #[test]
    fn roundtrip_and_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.prefs.json");
        assert_eq!(load(&path), Prefs::default());
        let prefs = Prefs {
            theme: "nord".into(),
        };
        save(&path, &prefs).unwrap();
        assert_eq!(load(&path), prefs);
        std::fs::write(&path, "pas du json").unwrap();
        assert_eq!(load(&path), Prefs::default());
    }
}
