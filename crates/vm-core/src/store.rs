/// Persistance de l'inventaire dans un fichier JSON local.
///
/// Chaque modification est transactionnelle : elle s'applique à une copie des
/// données, la copie est écrite sur le disque (fichier temporaire puis renommage
/// atomique), et seulement ensuite elle remplace l'état en mémoire. Une erreur de
/// validation ou d'écriture laisse donc mémoire et disque inchangés.
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::error::{CoreError, CoreResult};
use crate::models::*;
use crate::validation::{normalize_group, normalize_user, normalize_vm};

#[derive(Debug)]
pub struct Store {
    path: PathBuf,
    data: AppData,
    /// Copie de sauvegarde créée au chargement si le fichier était illisible
    recovered_backup: Option<PathBuf>,
}

impl Store {
    /// Ouvre le fichier de données, ou démarre un inventaire vide s'il n'existe pas.
    ///
    /// Un fichier corrompu n'est jamais écrasé : il est renommé en
    /// `data.json.corrupt-<horodatage>` avant de repartir de zéro.
    pub fn open(path: impl Into<PathBuf>) -> CoreResult<Store> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let mut recovered_backup = None;
        let data = if path.exists() {
            let content = fs::read_to_string(&path)?;
            match serde_json::from_str::<AppData>(&content) {
                Ok(data) if data.version > SCHEMA_VERSION => {
                    return Err(CoreError::Conflict(format!(
                        "Le fichier de données (format v{}) provient d'une version plus récente de l'application (format v{} pris en charge)",
                        data.version, SCHEMA_VERSION
                    )));
                }
                Ok(data) => data,
                Err(err) => {
                    let backup = backup_path(&path);
                    fs::rename(&path, &backup)?;
                    log::warn!("Fichier de données illisible ({err}), sauvegardé dans {backup:?}");
                    recovered_backup = Some(backup);
                    AppData::default()
                }
            }
        } else {
            AppData::default()
        };

        log::info!(
            "Inventaire chargé depuis {:?} : {} groupes, {} VM",
            path,
            data.groups.len(),
            data.vms.len()
        );
        Ok(Store {
            path,
            data,
            recovered_backup,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn data(&self) -> &AppData {
        &self.data
    }

    pub fn recovered_backup(&self) -> Option<&Path> {
        self.recovered_backup.as_deref()
    }

    /// Applique `change` sur une copie, l'écrit sur le disque puis la conserve
    fn commit<T>(&mut self, change: impl FnOnce(&mut AppData) -> CoreResult<T>) -> CoreResult<T> {
        let mut next = self.data.clone();
        let out = change(&mut next)?;
        next.version = SCHEMA_VERSION;
        write_atomic(&self.path, &next)?;
        self.data = next;
        Ok(out)
    }

    // ── Groupes ───────────────────────────────────────────────────────────

    pub fn groups(&self) -> &[Group] {
        &self.data.groups
    }

    pub fn create_group(&mut self, input: GroupInput) -> CoreResult<Group> {
        let input = normalize_group(input)?;
        self.commit(|data| {
            ensure_unique_group_name(data, &input.name, None)?;
            let group = Group::from_input(input);
            data.groups.push(group.clone());
            Ok(group)
        })
    }

    pub fn update_group(&mut self, id: &str, input: GroupInput) -> CoreResult<Group> {
        let input = normalize_group(input)?;
        self.commit(|data| {
            ensure_unique_group_name(data, &input.name, Some(id))?;
            let group = find_group_mut(data, id)?;
            group.apply(input);
            Ok(group.clone())
        })
    }

    /// Supprime un groupe ; ses VM sont conservées et deviennent « non classées ».
    /// Renvoie le nombre de VM détachées.
    pub fn delete_group(&mut self, id: &str) -> CoreResult<usize> {
        self.commit(|data| {
            let before = data.groups.len();
            data.groups.retain(|g| g.id != id);
            if data.groups.len() == before {
                return Err(CoreError::not_found("Groupe", id));
            }
            let mut detached = 0;
            for vm in data
                .vms
                .iter_mut()
                .filter(|v| v.group_id.as_deref() == Some(id))
            {
                vm.group_id = None;
                vm.touch();
                detached += 1;
            }
            Ok(detached)
        })
    }

    // ── VM ────────────────────────────────────────────────────────────────

    pub fn vms(&self) -> &[Vm] {
        &self.data.vms
    }

    pub fn vm(&self, id: &str) -> CoreResult<&Vm> {
        self.data
            .vms
            .iter()
            .find(|v| v.id == id)
            .ok_or_else(|| CoreError::not_found("VM", id))
    }

    pub fn create_vm(&mut self, input: VmInput) -> CoreResult<Vm> {
        let input = normalize_vm(input)?;
        self.commit(|data| {
            ensure_vm_refs(data, &input, None)?;
            let vm = Vm::from_input(input);
            data.vms.push(vm.clone());
            Ok(vm)
        })
    }

    pub fn update_vm(&mut self, id: &str, input: VmInput) -> CoreResult<Vm> {
        let input = normalize_vm(input)?;
        self.commit(|data| {
            ensure_vm_refs(data, &input, Some(id))?;
            let vm = find_vm_mut(data, id)?;
            vm.apply(input);
            Ok(vm.clone())
        })
    }

    pub fn delete_vm(&mut self, id: &str) -> CoreResult<()> {
        self.commit(|data| {
            let before = data.vms.len();
            data.vms.retain(|v| v.id != id);
            if data.vms.len() == before {
                return Err(CoreError::not_found("VM", id));
            }
            Ok(())
        })
    }

    /// Duplique une VM (réseau, SSH et utilisateurs compris) sous un nom libre
    pub fn duplicate_vm(&mut self, id: &str) -> CoreResult<Vm> {
        self.commit(|data| {
            let source = data
                .vms
                .iter()
                .find(|v| v.id == id)
                .ok_or_else(|| CoreError::not_found("VM", id))?
                .clone();
            let name = free_copy_name(data, &source.name);
            let now = Utc::now();
            let copy = Vm {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                users: source
                    .users
                    .iter()
                    .map(|u| VmUser {
                        id: uuid::Uuid::new_v4().to_string(),
                        ..u.clone()
                    })
                    .collect(),
                created_at: now,
                updated_at: now,
                ..source
            };
            data.vms.push(copy.clone());
            Ok(copy)
        })
    }

    // ── Utilisateurs d'une VM ─────────────────────────────────────────────

    pub fn add_user(&mut self, vm_id: &str, input: VmUserInput) -> CoreResult<VmUser> {
        let input = normalize_user(input)?;
        self.commit(|data| {
            let vm = find_vm_mut(data, vm_id)?;
            ensure_unique_username(vm, &input.username, None)?;
            let user = VmUser::from_input(input);
            vm.users.push(user.clone());
            vm.touch();
            Ok(user)
        })
    }

    pub fn update_user(
        &mut self,
        vm_id: &str,
        user_id: &str,
        input: VmUserInput,
    ) -> CoreResult<VmUser> {
        let input = normalize_user(input)?;
        self.commit(|data| {
            let vm = find_vm_mut(data, vm_id)?;
            ensure_unique_username(vm, &input.username, Some(user_id))?;
            let user = vm
                .users
                .iter_mut()
                .find(|u| u.id == user_id)
                .ok_or_else(|| CoreError::not_found("Utilisateur", user_id))?;
            user.apply(input);
            let user = user.clone();
            vm.touch();
            Ok(user)
        })
    }

    pub fn delete_user(&mut self, vm_id: &str, user_id: &str) -> CoreResult<()> {
        self.commit(|data| {
            let vm = find_vm_mut(data, vm_id)?;
            let before = vm.users.len();
            vm.users.retain(|u| u.id != user_id);
            if vm.users.len() == before {
                return Err(CoreError::not_found("Utilisateur", user_id));
            }
            vm.touch();
            Ok(())
        })
    }

    // ── Import / export ───────────────────────────────────────────────────

    pub fn export_to(&self, path: &Path) -> CoreResult<()> {
        write_atomic(path, &self.data)
    }

    /// Importe un fichier exporté. `replace` remplace tout l'inventaire ; sinon
    /// les groupes et VM sont ajoutés, en ignorant ceux dont l'identifiant ou le
    /// nom existe déjà. Renvoie le nombre de (groupes, VM) ajoutés.
    pub fn import_from(&mut self, path: &Path, replace: bool) -> CoreResult<(usize, usize)> {
        let content = fs::read_to_string(path)?;
        let incoming: AppData = serde_json::from_str(&content)?;
        if incoming.version > SCHEMA_VERSION {
            return Err(CoreError::Conflict(format!(
                "Ce fichier (format v{}) provient d'une version plus récente de l'application",
                incoming.version
            )));
        }
        self.commit(|data| {
            if replace {
                *data = AppData::default();
            }
            let (mut groups, mut vms) = (0, 0);
            for g in incoming.groups {
                let taken = data
                    .groups
                    .iter()
                    .any(|o| o.id == g.id || o.name.eq_ignore_ascii_case(&g.name));
                if !taken && normalize_group(group_input(&g)).is_ok() {
                    data.groups.push(g);
                    groups += 1;
                }
            }
            for mut vm in incoming.vms {
                let taken = data
                    .vms
                    .iter()
                    .any(|o| o.id == vm.id || o.name.eq_ignore_ascii_case(&vm.name));
                if taken || vm.name.trim().is_empty() {
                    continue;
                }
                // Un groupe absent du fichier ne doit pas laisser de référence orpheline
                if let Some(gid) = &vm.group_id {
                    if !data.groups.iter().any(|g| &g.id == gid) {
                        vm.group_id = None;
                    }
                }
                data.vms.push(vm);
                vms += 1;
            }
            Ok((groups, vms))
        })
    }
}

// ── Aides internes ────────────────────────────────────────────────────────

fn group_input(g: &Group) -> GroupInput {
    GroupInput {
        name: g.name.clone(),
        description: g.description.clone(),
        color: g.color.clone(),
    }
}

fn find_group_mut<'a>(data: &'a mut AppData, id: &str) -> CoreResult<&'a mut Group> {
    data.groups
        .iter_mut()
        .find(|g| g.id == id)
        .ok_or_else(|| CoreError::not_found("Groupe", id))
}

fn find_vm_mut<'a>(data: &'a mut AppData, id: &str) -> CoreResult<&'a mut Vm> {
    data.vms
        .iter_mut()
        .find(|v| v.id == id)
        .ok_or_else(|| CoreError::not_found("VM", id))
}

fn ensure_unique_group_name(data: &AppData, name: &str, except: Option<&str>) -> CoreResult<()> {
    let clash = data
        .groups
        .iter()
        .any(|g| Some(g.id.as_str()) != except && g.name.eq_ignore_ascii_case(name));
    if clash {
        return Err(CoreError::Conflict(format!(
            "Un groupe nommé « {name} » existe déjà"
        )));
    }
    Ok(())
}

fn ensure_vm_refs(data: &AppData, input: &VmInput, except: Option<&str>) -> CoreResult<()> {
    let clash = data
        .vms
        .iter()
        .any(|v| Some(v.id.as_str()) != except && v.name.eq_ignore_ascii_case(&input.name));
    if clash {
        return Err(CoreError::Conflict(format!(
            "Une VM nommée « {} » existe déjà",
            input.name
        )));
    }
    if let Some(gid) = &input.group_id {
        if !data.groups.iter().any(|g| &g.id == gid) {
            return Err(CoreError::not_found("Groupe", gid.clone()));
        }
    }
    Ok(())
}

fn ensure_unique_username(vm: &Vm, username: &str, except: Option<&str>) -> CoreResult<()> {
    let clash = vm
        .users
        .iter()
        .any(|u| Some(u.id.as_str()) != except && u.username == username);
    if clash {
        return Err(CoreError::Conflict(format!(
            "L'utilisateur « {username} » existe déjà sur {}",
            vm.name
        )));
    }
    Ok(())
}

fn free_copy_name(data: &AppData, base: &str) -> String {
    let taken = |n: &str| data.vms.iter().any(|v| v.name.eq_ignore_ascii_case(n));
    let first = format!("{base} (copie)");
    if !taken(&first) {
        return first;
    }
    (2..)
        .map(|i| format!("{base} (copie {i})"))
        .find(|n| !taken(n))
        .expect("itérateur infini")
}

fn backup_path(path: &Path) -> PathBuf {
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".corrupt-{stamp}"));
    path.with_file_name(name)
}

/// Écrit le JSON dans un fichier temporaire voisin puis le renomme : un arrêt
/// brutal pendant l'écriture ne laisse jamais un fichier de données tronqué
fn write_atomic(path: &Path, data: &AppData) -> CoreResult<()> {
    let content = serde_json::to_string_pretty(data)?;
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store() -> (TempDir, Store) {
        let dir = TempDir::new().unwrap();
        let s = Store::open(dir.path().join("data.json")).unwrap();
        (dir, s)
    }

    fn group(name: &str) -> GroupInput {
        GroupInput {
            name: name.into(),
            ..Default::default()
        }
    }

    fn vm(name: &str, group_id: Option<&str>) -> VmInput {
        VmInput {
            name: name.into(),
            group_id: group_id.map(String::from),
            ..Default::default()
        }
    }

    fn user(name: &str) -> VmUserInput {
        VmUserInput {
            username: name.into(),
            ..Default::default()
        }
    }

    #[test]
    fn data_survives_reopen() {
        let (dir, mut s) = store();
        let g = s.create_group(group("Prod")).unwrap();
        let v = s.create_vm(vm("web-01", Some(&g.id))).unwrap();
        s.add_user(&v.id, user("deploy")).unwrap();

        let reopened = Store::open(dir.path().join("data.json")).unwrap();
        assert_eq!(reopened.data(), s.data());
        assert_eq!(reopened.vms()[0].users[0].username, "deploy");
        assert!(!dir.path().join("data.json.tmp").exists());
    }

    #[test]
    fn group_crud_and_unique_names() {
        let (_d, mut s) = store();
        let g = s.create_group(group("Prod")).unwrap();
        assert!(matches!(
            s.create_group(group("prod")),
            Err(CoreError::Conflict(_))
        ));
        let other = s.create_group(group("Lab")).unwrap();
        assert!(s.update_group(&other.id, group("PROD")).is_err());
        let renamed = s.update_group(&g.id, group("Production")).unwrap();
        assert_eq!(renamed.name, "Production");
        assert!(renamed.updated_at >= renamed.created_at);
        // Renommer un groupe sous son propre nom (casse différente) reste permis
        assert!(s.update_group(&g.id, group("PRODUCTION")).is_ok());
        assert!(s.update_group("absent", group("x")).is_err());
    }

    #[test]
    fn deleting_a_group_detaches_its_vms() {
        let (_d, mut s) = store();
        let g = s.create_group(group("Prod")).unwrap();
        s.create_vm(vm("a", Some(&g.id))).unwrap();
        s.create_vm(vm("b", Some(&g.id))).unwrap();
        s.create_vm(vm("c", None)).unwrap();
        assert_eq!(s.delete_group(&g.id).unwrap(), 2);
        assert!(s.groups().is_empty());
        assert_eq!(s.vms().len(), 3);
        assert!(s.vms().iter().all(|v| v.group_id.is_none()));
        assert!(s.delete_group(&g.id).is_err());
    }

    #[test]
    fn vm_crud() {
        let (_d, mut s) = store();
        let v = s.create_vm(vm("web-01", None)).unwrap();
        assert!(s.create_vm(vm("WEB-01", None)).is_err());
        assert!(s.create_vm(vm("x", Some("absent"))).is_err());

        let mut edit = vm("web-02", None);
        edit.network.ip = "10.0.0.2".into();
        let updated = s.update_vm(&v.id, edit).unwrap();
        assert_eq!(updated.network.ip, "10.0.0.2");

        s.delete_vm(&v.id).unwrap();
        assert!(s.vms().is_empty());
        assert!(s.delete_vm(&v.id).is_err());
    }

    #[test]
    fn updating_a_vm_keeps_its_users() {
        let (_d, mut s) = store();
        let v = s.create_vm(vm("a", None)).unwrap();
        s.add_user(&v.id, user("root")).unwrap();
        let updated = s.update_vm(&v.id, vm("a", None)).unwrap();
        assert_eq!(updated.users.len(), 1);
    }

    #[test]
    fn failed_change_leaves_state_untouched() {
        let (dir, mut s) = store();
        s.create_vm(vm("a", None)).unwrap();
        let before = s.data().clone();
        assert!(s.create_vm(vm("A", None)).is_err());
        assert_eq!(s.data(), &before);
        let on_disk = Store::open(dir.path().join("data.json")).unwrap();
        assert_eq!(on_disk.data(), &before);
    }

    #[test]
    fn user_crud_and_unique_usernames() {
        let (_d, mut s) = store();
        let v = s.create_vm(vm("a", None)).unwrap();
        let root = s.add_user(&v.id, user("root")).unwrap();
        let deploy = s.add_user(&v.id, user("deploy")).unwrap();
        assert!(s.add_user(&v.id, user("root")).is_err());
        assert!(s.update_user(&v.id, &deploy.id, user("root")).is_err());

        let mut edit = user("deploy");
        edit.sudo = true;
        edit.role = "CI".into();
        let edited = s.update_user(&v.id, &deploy.id, edit).unwrap();
        assert!(edited.sudo);

        s.delete_user(&v.id, &root.id).unwrap();
        assert_eq!(s.vm(&v.id).unwrap().users.len(), 1);
        assert!(s.delete_user(&v.id, &root.id).is_err());
        assert!(s.add_user("absent", user("x")).is_err());
    }

    #[test]
    fn duplicate_vm_gets_free_name_and_new_ids() {
        let (_d, mut s) = store();
        let v = s.create_vm(vm("web", None)).unwrap();
        s.add_user(&v.id, user("root")).unwrap();
        let c1 = s.duplicate_vm(&v.id).unwrap();
        let c2 = s.duplicate_vm(&v.id).unwrap();
        assert_eq!(c1.name, "web (copie)");
        assert_eq!(c2.name, "web (copie 2)");
        assert_ne!(c1.id, v.id);
        assert_ne!(c1.users[0].id, s.vm(&v.id).unwrap().users[0].id);
        assert_eq!(c1.users[0].username, "root");
    }

    #[test]
    fn corrupt_file_is_backed_up_not_overwritten() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");
        fs::write(&path, "{ pas du json").unwrap();
        let s = Store::open(&path).unwrap();
        let backup = s
            .recovered_backup()
            .expect("sauvegarde attendue")
            .to_path_buf();
        assert_eq!(fs::read_to_string(backup).unwrap(), "{ pas du json");
        assert!(s.vms().is_empty());
    }

    #[test]
    fn newer_format_is_refused() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");
        fs::write(&path, r#"{"version": 999, "groups": [], "vms": []}"#).unwrap();
        assert!(Store::open(&path).is_err());
        // Le fichier reste intact
        assert!(fs::read_to_string(&path).unwrap().contains("999"));
    }

    #[test]
    fn export_then_import_merge_and_replace() {
        let (dir, mut s) = store();
        let g = s.create_group(group("Prod")).unwrap();
        s.create_vm(vm("a", Some(&g.id))).unwrap();
        let export = dir.path().join("export.json");
        s.export_to(&export).unwrap();

        // Fusion dans un inventaire qui contient déjà « a » : rien n'est dupliqué
        assert_eq!(s.import_from(&export, false).unwrap(), (0, 0));

        let (_d2, mut other) = store();
        other.create_vm(vm("b", None)).unwrap();
        assert_eq!(other.import_from(&export, false).unwrap(), (1, 1));
        assert_eq!(other.vms().len(), 2);

        assert_eq!(other.import_from(&export, true).unwrap(), (1, 1));
        assert_eq!(other.vms().len(), 1);
        assert_eq!(other.vms()[0].group_id.as_deref(), Some(g.id.as_str()));
    }

    #[test]
    fn import_drops_dangling_group_refs() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("in.json");
        let mut data = AppData::default();
        let mut v = Vm::from_input(vm("orphan", None));
        v.group_id = Some("ghost".into());
        data.vms.push(v);
        fs::write(&file, serde_json::to_string(&data).unwrap()).unwrap();

        let (_d, mut s) = store();
        s.import_from(&file, false).unwrap();
        assert_eq!(s.vms()[0].group_id, None);
    }

    #[test]
    fn loads_minimal_legacy_json() {
        // Champs facultatifs absents : les valeurs par défaut s'appliquent
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");
        fs::write(
            &path,
            r#"{"vms":[{"id":"1","name":"old","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}]}"#,
        )
        .unwrap();
        let s = Store::open(&path).unwrap();
        assert_eq!(s.vms()[0].ssh.port, 22);
        assert_eq!(s.data().version, SCHEMA_VERSION);
    }
}
