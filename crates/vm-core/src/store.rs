/// Persistance de l'inventaire dans un fichier JSON local.
///
/// Chaque modification est transactionnelle : elle s'applique à une copie des
/// données, la copie est écrite sur le disque (fichier temporaire puis renommage
/// atomique), et seulement ensuite elle remplace l'état en mémoire. Une erreur de
/// validation ou d'écriture laisse donc mémoire et disque inchangés.
///
/// Le fichier peut aussi être entièrement chiffré avec le mot de passe maître du
/// coffre : la clé reste alors en mémoire et chaque écriture rechiffre le tout.
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use zeroize::Zeroizing;

use crate::error::{CoreError, CoreResult};
use crate::models::*;
use crate::validation::{normalize_group, normalize_host, normalize_user, normalize_vm};
use crate::vault::{self, EncryptedFile, KdfParams, VaultHeader, VaultKey};

#[derive(Debug)]
pub struct Store {
    path: PathBuf,
    data: AppData,
    /// Copie de sauvegarde créée au chargement si le fichier était illisible
    recovered_backup: Option<PathBuf>,
    /// Clé de chiffrement du fichier, présente seulement si le fichier est chiffré
    file_key: Option<VaultKey>,
}

/// Bilan d'un import
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// Groupes ajoutés
    pub groups: usize,
    /// Machines physiques ajoutées
    pub hosts: usize,
    /// VM ajoutées
    pub vms: usize,
    /// Mots de passe écartés car chiffrés par un autre coffre (indéchiffrables ici)
    pub passwords_dropped: usize,
}

impl Store {
    /// Ouvre le fichier de données, ou démarre un inventaire vide s'il n'existe pas.
    ///
    /// Un fichier corrompu n'est jamais écrasé : il est renommé en
    /// `data.json.corrupt-<horodatage>` avant de repartir de zéro. Un fichier
    /// chiffré n'est pas corrompu : il donne `CoreError::Locked`, et s'ouvre avec
    /// [`Store::open_with_password`].
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
            if vault::is_encrypted_content(&content) {
                return Err(CoreError::Locked);
            }
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
            "Inventaire chargé depuis {:?} : {} groupes, {} machines physiques, {} VM",
            path,
            data.groups.len(),
            data.hosts.len(),
            data.vms.len()
        );
        Ok(Store {
            path,
            data,
            recovered_backup,
            file_key: None,
        })
    }

    /// Ouvre le fichier de données avec le mot de passe maître et renvoie aussi la
    /// clé du coffre. Un fichier chiffré est déchiffré et le reste à chaque
    /// écriture ; un fichier en clair s'ouvre normalement, puis le coffre est
    /// déverrouillé (il doit donc exister).
    pub fn open_with_password(
        path: impl Into<PathBuf>,
        master: &str,
    ) -> CoreResult<(Store, VaultKey)> {
        let path = path.into();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            if vault::is_encrypted_content(&content) {
                let (data, key) = decrypt_data(&content, master)?;
                log::info!(
                    "Inventaire chiffré chargé depuis {:?} : {} groupes, {} VM",
                    path,
                    data.groups.len(),
                    data.vms.len()
                );
                let store = Store {
                    path,
                    data,
                    recovered_backup: None,
                    file_key: Some(key.clone()),
                };
                return Ok((store, key));
            }
        }
        let store = Store::open(path)?;
        let key = store.unlock_vault(master)?;
        Ok((store, key))
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
        let file_key = self.file_key.clone();
        self.commit_keyed(file_key, change)
    }

    /// Comme `commit`, mais écrit le fichier avec `file_key` (`None` : en clair),
    /// qui devient la clé du fichier si l'écriture réussit
    fn commit_keyed<T>(
        &mut self,
        file_key: Option<VaultKey>,
        change: impl FnOnce(&mut AppData) -> CoreResult<T>,
    ) -> CoreResult<T> {
        let mut next = self.data.clone();
        let out = change(&mut next)?;
        next.version = SCHEMA_VERSION;
        let content = render(&next, file_key.as_ref())?;
        write_atomic(&self.path, &content)?;
        self.data = next;
        self.file_key = file_key;
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

    /// Supprime un groupe ; ses machines physiques et ses VM sont conservées et
    /// deviennent « non classées ». Renvoie le nombre de VM détachées.
    pub fn delete_group(&mut self, id: &str) -> CoreResult<usize> {
        self.commit(|data| {
            let before = data.groups.len();
            data.groups.retain(|g| g.id != id);
            if data.groups.len() == before {
                return Err(CoreError::not_found("Groupe", id));
            }
            for host in data
                .hosts
                .iter_mut()
                .filter(|h| h.group_id.as_deref() == Some(id))
            {
                host.group_id = None;
                host.touch();
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

    // ── Machines physiques ────────────────────────────────────────────────

    pub fn hosts(&self) -> &[Host] {
        &self.data.hosts
    }

    pub fn host(&self, id: &str) -> CoreResult<&Host> {
        self.data
            .hosts
            .iter()
            .find(|h| h.id == id)
            .ok_or_else(|| CoreError::not_found("Machine physique", id))
    }

    /// VM hébergées par une machine physique
    pub fn vms_on_host(&self, host_id: &str) -> Vec<&Vm> {
        self.data
            .vms
            .iter()
            .filter(|v| v.host_id.as_deref() == Some(host_id))
            .collect()
    }

    pub fn create_host(&mut self, input: HostInput) -> CoreResult<Host> {
        let input = normalize_host(input)?;
        self.commit(|data| {
            ensure_host_refs(data, &input, None)?;
            let host = Host::from_input(input);
            data.hosts.push(host.clone());
            Ok(host)
        })
    }

    /// Modifie une machine physique ; si son groupe change, toutes ses VM le suivent
    pub fn update_host(&mut self, id: &str, input: HostInput) -> CoreResult<Host> {
        let input = normalize_host(input)?;
        self.commit(|data| {
            ensure_host_refs(data, &input, Some(id))?;
            let host = find_host_mut(data, id)?;
            host.apply(input);
            let host = host.clone();
            for vm in data
                .vms
                .iter_mut()
                .filter(|v| v.host_id.as_deref() == Some(id) && v.group_id != host.group_id)
            {
                vm.group_id = host.group_id.clone();
                vm.touch();
            }
            Ok(host)
        })
    }

    /// Supprime une machine physique ; ses VM sont conservées, détachées de la
    /// machine mais dans leur groupe actuel. Renvoie le nombre de VM détachées.
    pub fn delete_host(&mut self, id: &str) -> CoreResult<usize> {
        self.commit(|data| {
            let before = data.hosts.len();
            data.hosts.retain(|h| h.id != id);
            if data.hosts.len() == before {
                return Err(CoreError::not_found("Machine physique", id));
            }
            let mut detached = 0;
            for vm in data
                .vms
                .iter_mut()
                .filter(|v| v.host_id.as_deref() == Some(id))
            {
                vm.host_id = None;
                vm.touch();
                detached += 1;
            }
            Ok(detached)
        })
    }

    /// Place une VM sur une machine physique (elle prend alors son groupe), ou
    /// l'en retire avec `None` (elle garde son groupe actuel)
    pub fn set_vm_host(&mut self, vm_id: &str, host_id: Option<&str>) -> CoreResult<Vm> {
        let host_id = host_id.map(str::trim).filter(|h| !h.is_empty());
        self.commit(|data| {
            let group_id = host_id.map(|h| host_group(data, h)).transpose()?;
            let vm = find_vm_mut(data, vm_id)?;
            vm.host_id = host_id.map(String::from);
            if let Some(group_id) = group_id {
                vm.group_id = group_id;
            }
            vm.touch();
            Ok(vm.clone())
        })
    }

    /// Place plusieurs VM sur une machine physique, tout ou rien : un seul
    /// identifiant inconnu annule l'ensemble. Renvoie le nombre de VM placées.
    pub fn assign_vms_to_host(&mut self, host_id: &str, vm_ids: &[String]) -> CoreResult<usize> {
        self.commit(|data| {
            let group_id = host_group(data, host_id)?;
            let mut placed: Vec<&str> = Vec::new();
            for vm_id in vm_ids {
                if placed.contains(&vm_id.as_str()) {
                    continue;
                }
                let vm = find_vm_mut(data, vm_id)?;
                if vm.host_id.as_deref() != Some(host_id) || vm.group_id != group_id {
                    vm.host_id = Some(host_id.to_string());
                    vm.group_id = group_id.clone();
                    vm.touch();
                }
                placed.push(vm_id);
            }
            Ok(placed.len())
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

    /// Crée une VM ; placée sur une machine physique, elle en prend le groupe
    pub fn create_vm(&mut self, input: VmInput) -> CoreResult<Vm> {
        let mut input = normalize_vm(input)?;
        self.commit(|data| {
            ensure_vm_refs(data, &mut input, None)?;
            let vm = Vm::from_input(input);
            data.vms.push(vm.clone());
            Ok(vm)
        })
    }

    /// Modifie une VM ; placée sur une machine physique, elle en prend le groupe
    pub fn update_vm(&mut self, id: &str, input: VmInput) -> CoreResult<Vm> {
        let mut input = normalize_vm(input)?;
        self.commit(|data| {
            ensure_vm_refs(data, &mut input, Some(id))?;
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

    /// Duplique une VM (réseau, SSH, utilisateurs et machine physique compris) sous un nom libre
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

    /// Exporte l'inventaire ; si le fichier de données est chiffré, l'export l'est
    /// aussi (même mot de passe maître)
    pub fn export_to(&self, path: &Path) -> CoreResult<()> {
        let content = render(&self.data, self.file_key.as_ref())?;
        write_atomic(path, &content)
    }

    /// Importe un fichier exporté, en clair ou chiffré (`master` est alors exigé,
    /// sinon `CoreError::Locked`). `replace` remplace tout l'inventaire ; sinon les
    /// groupes, machines physiques et VM sont ajoutés, en ignorant ceux dont
    /// l'identifiant ou le nom existe déjà. Les références vers un groupe ou une
    /// machine absents sont retirées, et chaque VM importée placée sur une machine
    /// prend le groupe de celle-ci.
    ///
    /// Les mots de passe importés ne sont gardés que s'ils restent déchiffrables :
    /// même coffre (même sel) que l'inventaire courant, ou remplacement complet
    /// qui reprend le coffre du fichier. Un inventaire chiffré garde toujours son
    /// propre coffre, puisque sa clé chiffre aussi le fichier.
    pub fn import_from(
        &mut self,
        path: &Path,
        replace: bool,
        master: Option<&str>,
    ) -> CoreResult<ImportReport> {
        let content = fs::read_to_string(path)?;
        let mut incoming: AppData = if vault::is_encrypted_content(&content) {
            let master = master.ok_or(CoreError::Locked)?;
            decrypt_data(&content, master)?.0
        } else {
            serde_json::from_str(&content)?
        };
        ensure_supported_version(&incoming, "Ce fichier")?;

        let incoming_vault = incoming.vault.take();
        let same_vault = matches!(
            (&self.data.vault, &incoming_vault),
            (Some(a), Some(b)) if a.salt == b.salt
        );
        let take_incoming_vault = replace && self.file_key.is_none();
        let keep_passwords = same_vault || take_incoming_vault;

        self.commit(|data| {
            if replace {
                let current_vault = data.vault.take();
                *data = AppData::default();
                data.vault = if take_incoming_vault {
                    incoming_vault
                } else {
                    current_vault
                };
            }
            let mut report = ImportReport::default();
            for g in incoming.groups {
                let taken = data
                    .groups
                    .iter()
                    .any(|o| o.id == g.id || o.name.eq_ignore_ascii_case(&g.name));
                if !taken && normalize_group(group_input(&g)).is_ok() {
                    data.groups.push(g);
                    report.groups += 1;
                }
            }
            for mut host in incoming.hosts {
                let taken = data
                    .hosts
                    .iter()
                    .any(|o| o.id == host.id || o.name.eq_ignore_ascii_case(&host.name));
                if taken || normalize_host(host_input(&host)).is_err() {
                    continue;
                }
                if let Some(gid) = &host.group_id {
                    if !data.groups.iter().any(|g| &g.id == gid) {
                        host.group_id = None;
                    }
                }
                data.hosts.push(host);
                report.hosts += 1;
            }
            for mut vm in incoming.vms {
                let taken = data
                    .vms
                    .iter()
                    .any(|o| o.id == vm.id || o.name.eq_ignore_ascii_case(&vm.name));
                if taken || vm.name.trim().is_empty() {
                    continue;
                }
                // Une machine ou un groupe absents ne doivent pas laisser de référence orpheline
                if let Some(hid) = &vm.host_id {
                    match data.hosts.iter().find(|h| &h.id == hid) {
                        Some(host) => vm.group_id = host.group_id.clone(),
                        None => vm.host_id = None,
                    }
                }
                if let Some(gid) = &vm.group_id {
                    if !data.groups.iter().any(|g| &g.id == gid) {
                        vm.group_id = None;
                    }
                }
                if !keep_passwords {
                    for user in &mut vm.users {
                        if user.password.take().is_some() {
                            report.passwords_dropped += 1;
                        }
                    }
                }
                data.vms.push(vm);
                report.vms += 1;
            }
            Ok(report)
        })
    }

    // ── Coffre des mots de passe ──────────────────────────────────────────

    pub fn vault(&self) -> Option<&VaultHeader> {
        self.data.vault.as_ref()
    }

    /// Crée le coffre avec le réglage Argon2id de production
    pub fn create_vault(&mut self, master: &str) -> CoreResult<VaultKey> {
        self.create_vault_with(master, KdfParams::default())
    }

    /// Crée le coffre avec un réglage Argon2id choisi (les tests en prennent un rapide)
    pub fn create_vault_with(&mut self, master: &str, params: KdfParams) -> CoreResult<VaultKey> {
        if self.data.vault.is_some() {
            return Err(CoreError::Conflict(
                "Un coffre de mots de passe existe déjà".into(),
            ));
        }
        let (header, key) = VaultHeader::create(master, params)?;
        self.commit(|data| {
            data.vault = Some(header);
            Ok(())
        })?;
        Ok(key)
    }

    /// Dérive la clé du coffre à partir du mot de passe maître
    pub fn unlock_vault(&self, master: &str) -> CoreResult<VaultKey> {
        self.require_vault()?.unlock(master)
    }

    /// Enregistre (ou, avec `None` ou une chaîne vide, efface) le mot de passe d'un utilisateur
    pub fn set_user_password(
        &mut self,
        key: &VaultKey,
        vm_id: &str,
        user_id: &str,
        password: Option<&str>,
    ) -> CoreResult<()> {
        self.checked_vault(key)?;
        let blob = match password.filter(|p| !p.is_empty()) {
            Some(p) => Some(vault::encrypt(key, p)?),
            None => None,
        };
        self.commit(|data| {
            let vm = find_vm_mut(data, vm_id)?;
            find_user_mut(vm, user_id)?.password = blob;
            vm.touch();
            Ok(())
        })
    }

    /// Déchiffre le mot de passe d'un utilisateur (`None` s'il n'en a pas)
    pub fn user_password(
        &self,
        key: &VaultKey,
        vm_id: &str,
        user_id: &str,
    ) -> CoreResult<Option<Zeroizing<String>>> {
        self.checked_vault(key)?;
        let user = self
            .vm(vm_id)?
            .users
            .iter()
            .find(|u| u.id == user_id)
            .ok_or_else(|| CoreError::not_found("Utilisateur", user_id))?;
        user.password
            .as_ref()
            .map(|blob| vault::decrypt(key, blob))
            .transpose()
    }

    /// Nombre de mots de passe enregistrés dans tout l'inventaire
    pub fn password_count(&self) -> usize {
        self.data
            .vms
            .iter()
            .flat_map(|v| &v.users)
            .filter(|u| u.password.is_some())
            .count()
    }

    /// Change le mot de passe maître : nouveau sel, nouvelle clé, et tous les mots
    /// de passe (ainsi que le fichier s'il est chiffré) sont rechiffrés d'un seul coup
    pub fn change_master_password(&mut self, old: &str, new: &str) -> CoreResult<VaultKey> {
        let current = self.require_vault()?;
        let old_key = current.unlock(old)?;
        let (header, new_key) = VaultHeader::create(new, current.kdf)?;
        let file_key = self.file_key.as_ref().map(|_| new_key.clone());
        self.commit_keyed(file_key, |data| {
            for user in data.vms.iter_mut().flat_map(|v| v.users.iter_mut()) {
                if let Some(blob) = &user.password {
                    let plain = vault::decrypt(&old_key, blob)?;
                    user.password = Some(vault::encrypt(&new_key, &plain)?);
                }
            }
            data.vault = Some(header);
            Ok(())
        })?;
        Ok(new_key)
    }

    /// Mot de passe maître oublié : supprime le coffre et tous les mots de passe
    /// enregistrés (irrécupérables). Renvoie le nombre de mots de passe supprimés.
    /// Refusé tant que le fichier est chiffré.
    pub fn reset_vault(&mut self) -> CoreResult<usize> {
        if self.is_file_encrypted() {
            return Err(CoreError::Conflict(
                "Désactivez d'abord le chiffrement du fichier avant de réinitialiser le coffre"
                    .into(),
            ));
        }
        self.commit(|data| {
            data.vault = None;
            let mut removed = 0;
            for vm in &mut data.vms {
                let mut touched = false;
                for user in &mut vm.users {
                    if user.password.take().is_some() {
                        removed += 1;
                        touched = true;
                    }
                }
                if touched {
                    vm.touch();
                }
            }
            Ok(removed)
        })
    }

    // ── Chiffrement du fichier de données ─────────────────────────────────

    pub fn is_file_encrypted(&self) -> bool {
        self.file_key.is_some()
    }

    /// Chiffre le fichier de données avec le mot de passe maître. Si le coffre
    /// existe, `master` doit le déverrouiller (un seul mot de passe pour tout) ;
    /// sinon le coffre est créé avec ce mot de passe.
    pub fn enable_file_encryption(&mut self, master: &str) -> CoreResult<VaultKey> {
        self.enable_file_encryption_with(master, KdfParams::default())
    }

    /// Comme [`Store::enable_file_encryption`] ; `params` ne sert que si le coffre est à créer
    pub fn enable_file_encryption_with(
        &mut self,
        master: &str,
        params: KdfParams,
    ) -> CoreResult<VaultKey> {
        if self.is_file_encrypted() {
            return Err(CoreError::Conflict(
                "Le fichier de données est déjà chiffré".into(),
            ));
        }
        let (new_header, key) = match &self.data.vault {
            Some(header) => (None, header.unlock(master)?),
            None => {
                let (header, key) = VaultHeader::create(master, params)?;
                (Some(header), key)
            }
        };
        self.commit_keyed(Some(key.clone()), |data| {
            if let Some(header) = new_header {
                data.vault = Some(header);
            }
            Ok(())
        })?;
        Ok(key)
    }

    /// Réécrit le fichier de données en clair (les mots de passe restent chiffrés par le coffre)
    pub fn disable_file_encryption(&mut self, master: &str) -> CoreResult<()> {
        if !self.is_file_encrypted() {
            return Err(CoreError::Conflict(
                "Le fichier de données n'est pas chiffré".into(),
            ));
        }
        self.unlock_vault(master)?;
        self.commit_keyed(None, |_| Ok(()))
    }

    fn require_vault(&self) -> CoreResult<&VaultHeader> {
        self.data.vault.as_ref().ok_or_else(|| {
            CoreError::Conflict("Aucun coffre de mots de passe n'a encore été créé".into())
        })
    }

    /// Vérifie que `key` ouvre bien le coffre actuel (une clé périmée après un
    /// changement de mot de passe maître écrirait des secrets illisibles)
    fn checked_vault(&self, key: &VaultKey) -> CoreResult<&VaultHeader> {
        let header = self.require_vault()?;
        if !header.verify(key) {
            return Err(CoreError::Conflict(
                "Clé du coffre périmée : déverrouillez à nouveau le coffre".into(),
            ));
        }
        Ok(header)
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

fn host_input(h: &Host) -> HostInput {
    HostInput {
        name: h.name.clone(),
        description: h.description.clone(),
        group_id: h.group_id.clone(),
        hypervisor: h.hypervisor.clone(),
        hostname: h.hostname.clone(),
        ip: h.ip.clone(),
        cpu: h.cpu.clone(),
        ram_gb: h.ram_gb,
        storage: h.storage.clone(),
        location: h.location.clone(),
        notes: h.notes.clone(),
    }
}

fn find_host_mut<'a>(data: &'a mut AppData, id: &str) -> CoreResult<&'a mut Host> {
    data.hosts
        .iter_mut()
        .find(|h| h.id == id)
        .ok_or_else(|| CoreError::not_found("Machine physique", id))
}

/// Groupe d'une machine physique (`NotFound` si elle n'existe pas)
fn host_group(data: &AppData, host_id: &str) -> CoreResult<Option<String>> {
    data.hosts
        .iter()
        .find(|h| h.id == host_id)
        .map(|h| h.group_id.clone())
        .ok_or_else(|| CoreError::not_found("Machine physique", host_id))
}

fn ensure_group_exists(data: &AppData, group_id: Option<&str>) -> CoreResult<()> {
    if let Some(gid) = group_id {
        if !data.groups.iter().any(|g| g.id == gid) {
            return Err(CoreError::not_found("Groupe", gid));
        }
    }
    Ok(())
}

fn ensure_host_refs(data: &AppData, input: &HostInput, except: Option<&str>) -> CoreResult<()> {
    let clash = data
        .hosts
        .iter()
        .any(|h| Some(h.id.as_str()) != except && h.name.eq_ignore_ascii_case(&input.name));
    if clash {
        return Err(CoreError::Conflict(format!(
            "Une machine physique nommée « {} » existe déjà",
            input.name
        )));
    }
    ensure_group_exists(data, input.group_id.as_deref())
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

/// Vérifie le nom et les références d'une VM ; si elle est placée sur une
/// machine physique, son groupe est remplacé par celui de la machine
fn ensure_vm_refs(data: &AppData, input: &mut VmInput, except: Option<&str>) -> CoreResult<()> {
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
    if let Some(hid) = &input.host_id {
        input.group_id = host_group(data, hid)?;
    }
    ensure_group_exists(data, input.group_id.as_deref())
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

fn ensure_supported_version(data: &AppData, what: &str) -> CoreResult<()> {
    if data.version > SCHEMA_VERSION {
        return Err(CoreError::Conflict(format!(
            "{what} (format v{}) provient d'une version plus récente de l'application",
            data.version
        )));
    }
    Ok(())
}

/// Déchiffre une enveloppe et vérifie que le coffre qu'elle contient correspond à sa clé
fn decrypt_data(content: &str, master: &str) -> CoreResult<(AppData, VaultKey)> {
    let (plain, key) = EncryptedFile::parse(content)?.open(master)?;
    let data: AppData = serde_json::from_slice(&plain)?;
    ensure_supported_version(&data, "Le fichier de données")?;
    if !data.vault.as_ref().is_some_and(|v| v.verify(&key)) {
        return Err(CoreError::Crypto(
            "le coffre ne correspond pas au fichier chiffré".into(),
        ));
    }
    Ok((data, key))
}

/// Sérialise l'inventaire, chiffré dans une enveloppe si `file_key` est fourni
fn render(data: &AppData, file_key: Option<&VaultKey>) -> CoreResult<String> {
    let Some(key) = file_key else {
        return Ok(serde_json::to_string_pretty(data)?);
    };
    let header = data.vault.as_ref().ok_or_else(|| {
        CoreError::Conflict("Le chiffrement du fichier exige un coffre de mots de passe".into())
    })?;
    let plain = Zeroizing::new(serde_json::to_vec(data)?);
    let envelope = EncryptedFile::seal(header, key, &plain)?;
    Ok(serde_json::to_string_pretty(&envelope)?)
}

fn find_user_mut<'a>(vm: &'a mut Vm, id: &str) -> CoreResult<&'a mut VmUser> {
    vm.users
        .iter_mut()
        .find(|u| u.id == id)
        .ok_or_else(|| CoreError::not_found("Utilisateur", id))
}

/// Écrit le contenu dans un fichier temporaire voisin puis le renomme : un arrêt
/// brutal pendant l'écriture ne laisse jamais un fichier de données tronqué
fn write_atomic(path: &Path, content: &str) -> CoreResult<()> {
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
        assert_eq!(s.import_from(&export, false, None).unwrap(), report(0, 0));

        let (_d2, mut other) = store();
        other.create_vm(vm("b", None)).unwrap();
        assert_eq!(
            other.import_from(&export, false, None).unwrap(),
            report(1, 1)
        );
        assert_eq!(other.vms().len(), 2);

        assert_eq!(
            other.import_from(&export, true, None).unwrap(),
            report(1, 1)
        );
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
        s.import_from(&file, false, None).unwrap();
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

    // ── Coffre des mots de passe ──────────────────────────────────────────

    const MASTER: &str = "maître-secret";

    fn report(groups: usize, vms: usize) -> ImportReport {
        ImportReport {
            groups,
            hosts: 0,
            vms,
            passwords_dropped: 0,
        }
    }

    fn fast() -> KdfParams {
        KdfParams::insecure_for_tests()
    }

    /// Inventaire avec un coffre, une VM « web » et un utilisateur « root » protégé
    fn store_with_password() -> (TempDir, Store, VaultKey, String, String) {
        let (dir, mut s) = store();
        let key = s.create_vault_with(MASTER, fast()).unwrap();
        let v = s.create_vm(vm("web", None)).unwrap();
        let u = s.add_user(&v.id, user("root")).unwrap();
        s.set_user_password(&key, &v.id, &u.id, Some("hunter2"))
            .unwrap();
        (dir, s, key, v.id, u.id)
    }

    fn pw(s: &Store, key: &VaultKey, vm_id: &str, user_id: &str) -> Option<String> {
        s.user_password(key, vm_id, user_id)
            .unwrap()
            .map(|p| p.to_string())
    }

    #[test]
    fn legacy_json_without_vault_fields_loads() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");
        fs::write(
            &path,
            r#"{"version":1,"groups":[],"vms":[{"id":"1","name":"old","users":[{"id":"u","username":"root"}],"created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}]}"#,
        )
        .unwrap();
        let s = Store::open(&path).unwrap();
        assert!(s.vault().is_none());
        assert_eq!(s.vms()[0].users[0].password, None);
        assert_eq!(s.password_count(), 0);
        // Sans coffre ni mot de passe, rien de nouveau n'apparaît dans le JSON
        let json = serde_json::to_string(s.data()).unwrap();
        assert!(!json.contains("vault") && !json.contains("password"));
    }

    #[test]
    fn create_and_unlock_vault() {
        let (_d, mut s) = store();
        assert!(s.unlock_vault(MASTER).is_err());
        assert!(matches!(
            s.create_vault_with("court", fast()),
            Err(CoreError::Validation(_))
        ));
        s.create_vault_with(MASTER, fast()).unwrap();
        assert!(s.vault().is_some());
        assert!(matches!(
            s.create_vault_with(MASTER, fast()),
            Err(CoreError::Conflict(_))
        ));
        assert!(s.unlock_vault(MASTER).is_ok());
        assert!(matches!(
            s.unlock_vault("mauvais-mdp"),
            Err(CoreError::WrongPassword)
        ));
    }

    #[test]
    fn set_get_and_remove_user_password() {
        let (dir, mut s, key, vm_id, user_id) = store_with_password();
        assert_eq!(pw(&s, &key, &vm_id, &user_id).as_deref(), Some("hunter2"));
        assert_eq!(s.password_count(), 1);
        // Jamais en clair sur le disque
        let raw = fs::read_to_string(dir.path().join("data.json")).unwrap();
        assert!(!raw.contains("hunter2"));

        // Modifier l'utilisateur conserve son mot de passe
        let mut edit = user("root");
        edit.sudo = true;
        s.update_user(&vm_id, &user_id, edit).unwrap();
        assert_eq!(pw(&s, &key, &vm_id, &user_id).as_deref(), Some("hunter2"));

        let before = s.vm(&vm_id).unwrap().updated_at;
        s.set_user_password(&key, &vm_id, &user_id, Some(""))
            .unwrap();
        assert_eq!(pw(&s, &key, &vm_id, &user_id), None);
        assert!(s.vm(&vm_id).unwrap().updated_at >= before);
        s.set_user_password(&key, &vm_id, &user_id, Some("x"))
            .unwrap();
        s.set_user_password(&key, &vm_id, &user_id, None).unwrap();
        assert_eq!(s.password_count(), 0);

        assert!(s
            .set_user_password(&key, &vm_id, "absent", Some("x"))
            .is_err());
        assert!(s.user_password(&key, "absent", &user_id).is_err());
    }

    #[test]
    fn foreign_key_is_refused() {
        let (_d, mut s, _key, vm_id, user_id) = store_with_password();
        let (_d2, mut other) = store();
        let foreign = other.create_vault_with(MASTER, fast()).unwrap();
        assert!(s.user_password(&foreign, &vm_id, &user_id).is_err());
        assert!(s
            .set_user_password(&foreign, &vm_id, &user_id, Some("x"))
            .is_err());
    }

    #[test]
    fn passwords_persist_across_reopen() {
        let (dir, s, _key, vm_id, user_id) = store_with_password();
        drop(s);
        let reopened = Store::open(dir.path().join("data.json")).unwrap();
        let key = reopened.unlock_vault(MASTER).unwrap();
        assert_eq!(
            pw(&reopened, &key, &vm_id, &user_id).as_deref(),
            Some("hunter2")
        );
        let (again, key2) =
            Store::open_with_password(dir.path().join("data.json"), MASTER).unwrap();
        assert_eq!(
            pw(&again, &key2, &vm_id, &user_id).as_deref(),
            Some("hunter2")
        );
    }

    #[test]
    fn change_master_password_reencrypts_everything() {
        let (dir, mut s, old_key, vm_id, user_id) = store_with_password();
        assert!(s
            .change_master_password("mauvais-mdp", "nouveau-mdp")
            .is_err());
        assert!(s.change_master_password(MASTER, "court").is_err());
        let new_key = s.change_master_password(MASTER, "nouveau-mdp").unwrap();

        assert!(matches!(
            s.unlock_vault(MASTER),
            Err(CoreError::WrongPassword)
        ));
        assert!(s.user_password(&old_key, &vm_id, &user_id).is_err());
        assert_eq!(
            pw(&s, &new_key, &vm_id, &user_id).as_deref(),
            Some("hunter2")
        );
        let reopened = Store::open(dir.path().join("data.json")).unwrap();
        let key = reopened.unlock_vault("nouveau-mdp").unwrap();
        assert_eq!(
            pw(&reopened, &key, &vm_id, &user_id).as_deref(),
            Some("hunter2")
        );
    }

    #[test]
    fn reset_vault_removes_all_passwords() {
        let (_d, mut s, key, vm_id, _user_id) = store_with_password();
        let other = s.add_user(&vm_id, user("deploy")).unwrap();
        s.set_user_password(&key, &vm_id, &other.id, Some("pw"))
            .unwrap();
        assert_eq!(s.reset_vault().unwrap(), 2);
        assert!(s.vault().is_none());
        assert_eq!(s.password_count(), 0);
        assert_eq!(s.vm(&vm_id).unwrap().users.len(), 2);
        // Un nouveau coffre peut être créé ensuite
        s.create_vault_with("autre-maître", fast()).unwrap();
    }

    #[test]
    fn duplicate_vm_keeps_passwords() {
        let (_d, mut s, key, vm_id, _user_id) = store_with_password();
        let copy = s.duplicate_vm(&vm_id).unwrap();
        assert_eq!(
            pw(&s, &key, &copy.id, &copy.users[0].id).as_deref(),
            Some("hunter2")
        );
        assert_eq!(s.password_count(), 2);
    }

    #[test]
    fn import_merge_strips_foreign_passwords() {
        let (dir, s, _key, _vm_id, _user_id) = store_with_password();
        let export = dir.path().join("export.json");
        s.export_to(&export).unwrap();

        // Coffre différent (autre sel) : mots de passe indéchiffrables, donc écartés
        let (_d2, mut other) = store();
        other.create_vault_with(MASTER, fast()).unwrap();
        let r = other.import_from(&export, false, None).unwrap();
        assert_eq!((r.vms, r.passwords_dropped), (1, 1));
        assert_eq!(other.password_count(), 0);

        // Aucun coffre local : écartés aussi
        let (_d3, mut bare) = store();
        let r = bare.import_from(&export, false, None).unwrap();
        assert_eq!(r.passwords_dropped, 1);
        assert!(bare.vault().is_none());
    }

    #[test]
    fn import_merge_keeps_passwords_of_same_vault() {
        let (dir, mut s, key, vm_id, _user_id) = store_with_password();
        let export = dir.path().join("export.json");
        s.export_to(&export).unwrap();
        s.delete_vm(&vm_id).unwrap();
        let r = s.import_from(&export, false, None).unwrap();
        assert_eq!((r.vms, r.passwords_dropped), (1, 0));
        let v = &s.vms()[0];
        assert_eq!(
            pw(&s, &key, &v.id, &v.users[0].id).as_deref(),
            Some("hunter2")
        );
    }

    #[test]
    fn import_replace_takes_incoming_vault() {
        let (dir, s, _key, vm_id, user_id) = store_with_password();
        let export = dir.path().join("export.json");
        s.export_to(&export).unwrap();

        let (_d2, mut other) = store();
        other.create_vault_with("autre-maître", fast()).unwrap();
        let r = other.import_from(&export, true, None).unwrap();
        assert_eq!(r.passwords_dropped, 0);
        assert_eq!(other.vault(), s.vault());
        let key = other.unlock_vault(MASTER).unwrap();
        assert_eq!(
            pw(&other, &key, &vm_id, &user_id).as_deref(),
            Some("hunter2")
        );
    }

    // ── Chiffrement du fichier ────────────────────────────────────────────

    #[test]
    fn enable_encryption_locks_the_file() {
        let (dir, mut s, _key, vm_id, user_id) = store_with_password();
        let path = dir.path().join("data.json");
        assert!(!s.is_file_encrypted());
        assert!(s.enable_file_encryption("mauvais-mdp").is_err());
        assert!(!s.is_file_encrypted());
        s.enable_file_encryption(MASTER).unwrap();
        assert!(s.is_file_encrypted());
        assert!(matches!(
            s.enable_file_encryption(MASTER),
            Err(CoreError::Conflict(_))
        ));

        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("vm-manager-encrypted"));
        assert!(!raw.contains("web") && !raw.contains("root"));

        // Pas de sauvegarde « corrompue » : le fichier est simplement verrouillé
        assert!(matches!(Store::open(&path), Err(CoreError::Locked)));
        assert!(path.exists());
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
            .collect();
        assert!(leftovers.is_empty());

        assert!(matches!(
            Store::open_with_password(&path, "mauvais-mdp"),
            Err(CoreError::WrongPassword)
        ));
        let (reopened, key) = Store::open_with_password(&path, MASTER).unwrap();
        assert!(reopened.is_file_encrypted());
        assert_eq!(reopened.data(), s.data());
        assert_eq!(
            pw(&reopened, &key, &vm_id, &user_id).as_deref(),
            Some("hunter2")
        );
    }

    #[test]
    fn enable_encryption_creates_missing_vault() {
        let (dir, mut s) = store();
        s.create_vm(vm("web", None)).unwrap();
        assert!(s.enable_file_encryption_with("court", fast()).is_err());
        let key = s.enable_file_encryption_with(MASTER, fast()).unwrap();
        assert!(s.vault().unwrap().verify(&key));
        let (reopened, _) =
            Store::open_with_password(dir.path().join("data.json"), MASTER).unwrap();
        assert_eq!(reopened.vms()[0].name, "web");
    }

    #[test]
    fn edits_after_unlock_stay_encrypted() {
        let (dir, mut s, _key, _vm_id, _user_id) = store_with_password();
        let path = dir.path().join("data.json");
        s.enable_file_encryption(MASTER).unwrap();
        drop(s);

        let (mut s, key) = Store::open_with_password(&path, MASTER).unwrap();
        let first_nonce = EncryptedFile::parse(&fs::read_to_string(&path).unwrap())
            .unwrap()
            .nonce;
        let v = s.create_vm(vm("db-secret", None)).unwrap();
        let u = s.add_user(&v.id, user("postgres")).unwrap();
        s.set_user_password(&key, &v.id, &u.id, Some("pg-pass"))
            .unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("db-secret") && !raw.contains("postgres"));
        assert_ne!(EncryptedFile::parse(&raw).unwrap().nonce, first_nonce);
        let (again, key) = Store::open_with_password(&path, MASTER).unwrap();
        assert_eq!(pw(&again, &key, &v.id, &u.id).as_deref(), Some("pg-pass"));
    }

    #[test]
    fn disable_encryption_writes_plaintext() {
        let (dir, mut s, _key, vm_id, user_id) = store_with_password();
        let path = dir.path().join("data.json");
        assert!(s.disable_file_encryption(MASTER).is_err());
        s.enable_file_encryption(MASTER).unwrap();
        assert!(s.reset_vault().is_err());
        assert!(s.disable_file_encryption("mauvais-mdp").is_err());
        assert!(s.is_file_encrypted());
        s.disable_file_encryption(MASTER).unwrap();
        assert!(!s.is_file_encrypted());

        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("web") && !raw.contains("hunter2"));
        let reopened = Store::open(&path).unwrap();
        let key = reopened.unlock_vault(MASTER).unwrap();
        assert_eq!(
            pw(&reopened, &key, &vm_id, &user_id).as_deref(),
            Some("hunter2")
        );
    }

    #[test]
    fn change_master_rekeys_the_file() {
        let (dir, mut s, _key, vm_id, user_id) = store_with_password();
        let path = dir.path().join("data.json");
        s.enable_file_encryption(MASTER).unwrap();
        s.change_master_password(MASTER, "nouveau-mdp").unwrap();
        assert!(s.is_file_encrypted());

        assert!(matches!(
            Store::open_with_password(&path, MASTER),
            Err(CoreError::WrongPassword)
        ));
        let (mut reopened, key) = Store::open_with_password(&path, "nouveau-mdp").unwrap();
        assert_eq!(
            pw(&reopened, &key, &vm_id, &user_id).as_deref(),
            Some("hunter2")
        );
        // Les écritures suivantes utilisent bien la nouvelle clé
        reopened.create_vm(vm("après", None)).unwrap();
        assert!(Store::open_with_password(&path, "nouveau-mdp").is_ok());
    }

    #[test]
    fn encrypted_export_import_roundtrip() {
        let (dir, mut s, _key, vm_id, user_id) = store_with_password();
        s.enable_file_encryption(MASTER).unwrap();
        let export = dir.path().join("export.json");
        s.export_to(&export).unwrap();
        let raw = fs::read_to_string(&export).unwrap();
        assert!(raw.contains("vm-manager-encrypted") && !raw.contains("web"));

        let (_d2, mut other) = store();
        assert!(matches!(
            other.import_from(&export, true, None),
            Err(CoreError::Locked)
        ));
        assert!(matches!(
            other.import_from(&export, true, Some("mauvais-mdp")),
            Err(CoreError::WrongPassword)
        ));
        let r = other.import_from(&export, true, Some(MASTER)).unwrap();
        assert_eq!((r.vms, r.passwords_dropped), (1, 0));
        assert!(!other.is_file_encrypted());
        let key = other.unlock_vault(MASTER).unwrap();
        assert_eq!(
            pw(&other, &key, &vm_id, &user_id).as_deref(),
            Some("hunter2")
        );

        // Un inventaire chiffré garde son coffre lors d'un remplacement par un coffre étranger
        let (_d3, mut locked) = store();
        let own = locked
            .enable_file_encryption_with("autre-maître", fast())
            .unwrap();
        let r = locked.import_from(&export, true, Some(MASTER)).unwrap();
        assert_eq!((r.vms, r.passwords_dropped), (1, 1));
        assert!(locked.vault().unwrap().verify(&own));
    }

    // ── Machines physiques ────────────────────────────────────────────────

    fn host(name: &str, group_id: Option<&str>) -> HostInput {
        HostInput {
            name: name.into(),
            group_id: group_id.map(String::from),
            ..Default::default()
        }
    }

    fn vm_on(name: &str, host_id: &str, group_id: Option<&str>) -> VmInput {
        VmInput {
            host_id: Some(host_id.into()),
            ..vm(name, group_id)
        }
    }

    #[test]
    fn host_crud_and_unique_names() {
        let (dir, mut s) = store();
        let g = s.create_group(group("Prod")).unwrap();
        let mut input = host(" pve-01 ", Some(&g.id));
        input.ip = "10.0.0.2".into();
        input.ram_gb = Some(64);
        let h = s.create_host(input).unwrap();
        assert_eq!(h.name, "pve-01");
        assert_eq!(s.host(&h.id).unwrap().ram_gb, Some(64));
        assert!(matches!(
            s.create_host(host("PVE-01", None)),
            Err(CoreError::Conflict(_))
        ));
        assert!(matches!(
            s.create_host(host("x", Some("absent"))),
            Err(CoreError::NotFound { .. })
        ));
        let mut bad = host("y", None);
        bad.ip = "pas-une-ip".into();
        assert!(matches!(s.create_host(bad), Err(CoreError::Validation(_))));

        let other = s.create_host(host("esx-01", None)).unwrap();
        assert!(s.update_host(&other.id, host("pve-01", None)).is_err());
        let renamed = s.update_host(&h.id, host("PVE-01", Some(&g.id))).unwrap();
        assert_eq!(renamed.name, "PVE-01");
        assert!(renamed.updated_at >= renamed.created_at);
        assert!(s.update_host("absent", host("z", None)).is_err());

        let reopened = Store::open(dir.path().join("data.json")).unwrap();
        assert_eq!(reopened.hosts(), s.hosts());

        assert_eq!(s.delete_host(&other.id).unwrap(), 0);
        assert_eq!(s.hosts().len(), 1);
        assert!(matches!(
            s.delete_host(&other.id),
            Err(CoreError::NotFound { .. })
        ));
        assert!(s.host(&other.id).is_err());
    }

    #[test]
    fn vm_on_host_takes_the_host_group() {
        let (_d, mut s) = store();
        let prod = s.create_group(group("Prod")).unwrap();
        let lab = s.create_group(group("Lab")).unwrap();
        let h = s.create_host(host("pve-01", Some(&prod.id))).unwrap();

        // Création : le groupe saisi est remplacé par celui de la machine
        let v = s.create_vm(vm_on("a", &h.id, Some(&lab.id))).unwrap();
        assert_eq!(v.host_id.as_deref(), Some(h.id.as_str()));
        assert_eq!(v.group_id.as_deref(), Some(prod.id.as_str()));

        // Modification : idem
        let v = s.update_vm(&v.id, vm_on("a", &h.id, None)).unwrap();
        assert_eq!(v.group_id.as_deref(), Some(prod.id.as_str()));

        // Sans machine, la VM garde son propre groupe
        let v = s.update_vm(&v.id, vm("a", Some(&lab.id))).unwrap();
        assert_eq!(v.host_id, None);
        assert_eq!(v.group_id.as_deref(), Some(lab.id.as_str()));

        // Machine inconnue
        let before = s.data().clone();
        assert!(matches!(
            s.create_vm(vm_on("b", "absent", None)),
            Err(CoreError::NotFound {
                kind: "Machine physique",
                ..
            })
        ));
        assert!(s.update_vm(&v.id, vm_on("a", "absent", None)).is_err());
        assert_eq!(s.data(), &before);

        // Machine sans groupe : la VM devient non classée
        let bare = s.create_host(host("bare", None)).unwrap();
        let w = s.create_vm(vm_on("w", &bare.id, Some(&lab.id))).unwrap();
        assert_eq!(w.group_id, None);
        assert_eq!(s.vms_on_host(&bare.id).len(), 1);
        assert_eq!(s.vms_on_host(&h.id).len(), 0);
    }

    #[test]
    fn set_vm_host_applies_and_removes_the_host() {
        let (_d, mut s) = store();
        let prod = s.create_group(group("Prod")).unwrap();
        let lab = s.create_group(group("Lab")).unwrap();
        let h = s.create_host(host("pve-01", Some(&prod.id))).unwrap();
        let v = s.create_vm(vm("a", Some(&lab.id))).unwrap();

        let placed = s.set_vm_host(&v.id, Some(&h.id)).unwrap();
        assert_eq!(placed.host_id.as_deref(), Some(h.id.as_str()));
        assert_eq!(placed.group_id.as_deref(), Some(prod.id.as_str()));
        assert_eq!(s.vm(&v.id).unwrap(), &placed);

        // Retirée de la machine : elle garde le groupe de la machine
        let removed = s.set_vm_host(&v.id, None).unwrap();
        assert_eq!(removed.host_id, None);
        assert_eq!(removed.group_id.as_deref(), Some(prod.id.as_str()));

        assert!(matches!(
            s.set_vm_host(&v.id, Some("absent")),
            Err(CoreError::NotFound { .. })
        ));
        assert!(matches!(
            s.set_vm_host("absent", Some(&h.id)),
            Err(CoreError::NotFound { .. })
        ));
    }

    #[test]
    fn assign_vms_to_host_is_all_or_nothing() {
        let (_d, mut s) = store();
        let prod = s.create_group(group("Prod")).unwrap();
        let h = s.create_host(host("pve-01", Some(&prod.id))).unwrap();
        let a = s.create_vm(vm("a", None)).unwrap();
        let b = s.create_vm(vm("b", None)).unwrap();

        let before = s.data().clone();
        let ids = vec![a.id.clone(), "absent".to_string()];
        assert!(matches!(
            s.assign_vms_to_host(&h.id, &ids),
            Err(CoreError::NotFound { .. })
        ));
        assert_eq!(s.data(), &before);
        assert!(s
            .assign_vms_to_host("absent", std::slice::from_ref(&a.id))
            .is_err());

        let ids = vec![a.id.clone(), b.id.clone(), a.id.clone()];
        assert_eq!(s.assign_vms_to_host(&h.id, &ids).unwrap(), 2);
        for v in s.vms() {
            assert_eq!(v.host_id.as_deref(), Some(h.id.as_str()));
            assert_eq!(v.group_id.as_deref(), Some(prod.id.as_str()));
        }
        assert_eq!(s.vms_on_host(&h.id).len(), 2);
        assert_eq!(s.assign_vms_to_host(&h.id, &[]).unwrap(), 0);
    }

    #[test]
    fn changing_host_group_moves_its_vms() {
        let (_d, mut s) = store();
        let prod = s.create_group(group("Prod")).unwrap();
        let lab = s.create_group(group("Lab")).unwrap();
        let h = s.create_host(host("pve-01", Some(&prod.id))).unwrap();
        let a = s.create_vm(vm_on("a", &h.id, None)).unwrap();
        let b = s.create_vm(vm_on("b", &h.id, None)).unwrap();
        let other = s.create_vm(vm("c", Some(&prod.id))).unwrap();

        s.update_host(&h.id, host("pve-01", Some(&lab.id))).unwrap();
        for id in [&a.id, &b.id] {
            let v = s.vm(id).unwrap();
            assert_eq!(v.group_id.as_deref(), Some(lab.id.as_str()));
            assert!(v.updated_at >= a.updated_at);
        }
        // Une VM hors de la machine n'est pas touchée
        assert_eq!(s.vm(&other.id).unwrap(), &other);

        s.update_host(&h.id, host("pve-01", None)).unwrap();
        assert!(s.vms_on_host(&h.id).iter().all(|v| v.group_id.is_none()));
    }

    #[test]
    fn deleting_a_host_detaches_vms_keeping_their_group() {
        let (_d, mut s) = store();
        let prod = s.create_group(group("Prod")).unwrap();
        let h = s.create_host(host("pve-01", Some(&prod.id))).unwrap();
        s.create_vm(vm_on("a", &h.id, None)).unwrap();
        s.create_vm(vm_on("b", &h.id, None)).unwrap();
        s.create_vm(vm("c", None)).unwrap();

        assert_eq!(s.delete_host(&h.id).unwrap(), 2);
        assert!(s.hosts().is_empty());
        assert_eq!(s.vms().len(), 3);
        assert!(s.vms().iter().all(|v| v.host_id.is_none()));
        let grouped = s
            .vms()
            .iter()
            .filter(|v| v.group_id.as_deref() == Some(prod.id.as_str()))
            .count();
        assert_eq!(grouped, 2);
    }

    #[test]
    fn deleting_a_group_ungroups_its_hosts_and_their_vms() {
        let (_d, mut s) = store();
        let prod = s.create_group(group("Prod")).unwrap();
        let lab = s.create_group(group("Lab")).unwrap();
        let h = s.create_host(host("pve-01", Some(&prod.id))).unwrap();
        let keep = s.create_host(host("esx-01", Some(&lab.id))).unwrap();
        s.create_vm(vm_on("a", &h.id, None)).unwrap();
        s.create_vm(vm("b", Some(&prod.id))).unwrap();
        s.create_vm(vm_on("c", &keep.id, None)).unwrap();

        assert_eq!(s.delete_group(&prod.id).unwrap(), 2);
        assert_eq!(s.host(&h.id).unwrap().group_id, None);
        assert_eq!(
            s.host(&keep.id).unwrap().group_id.as_deref(),
            Some(lab.id.as_str())
        );
        // Les VM restent sur leur machine
        assert_eq!(s.vms_on_host(&h.id)[0].group_id, None);
        assert_eq!(
            s.vms_on_host(&keep.id)[0].group_id.as_deref(),
            Some(lab.id.as_str())
        );
    }

    #[test]
    fn duplicate_vm_keeps_its_host() {
        let (_d, mut s) = store();
        let prod = s.create_group(group("Prod")).unwrap();
        let h = s.create_host(host("pve-01", Some(&prod.id))).unwrap();
        let v = s.create_vm(vm_on("web", &h.id, None)).unwrap();
        let copy = s.duplicate_vm(&v.id).unwrap();
        assert_eq!(copy.host_id.as_deref(), Some(h.id.as_str()));
        assert_eq!(copy.group_id.as_deref(), Some(prod.id.as_str()));
        assert_eq!(s.vms_on_host(&h.id).len(), 2);
    }

    #[test]
    fn import_merges_and_replaces_hosts() {
        let (dir, mut s) = store();
        let g = s.create_group(group("Prod")).unwrap();
        let h = s.create_host(host("pve-01", Some(&g.id))).unwrap();
        s.create_vm(vm_on("a", &h.id, None)).unwrap();
        let export = dir.path().join("export.json");
        s.export_to(&export).unwrap();

        // Fusion dans l'inventaire d'origine : rien n'est dupliqué
        let r = s.import_from(&export, false, None).unwrap();
        assert_eq!((r.groups, r.hosts, r.vms), (0, 0, 0));

        // Une machine de même nom (autre identifiant) bloque l'import de la machine,
        // et la VM qui la référençait est détachée
        let (_d2, mut other) = store();
        other.create_host(host("PVE-01", None)).unwrap();
        let r = other.import_from(&export, false, None).unwrap();
        assert_eq!((r.groups, r.hosts, r.vms), (1, 0, 1));
        let a = other.vms().iter().find(|v| v.name == "a").unwrap();
        assert_eq!(a.host_id, None);
        assert_eq!(a.group_id.as_deref(), Some(g.id.as_str()));

        // Fusion dans un inventaire vide
        let (_d3, mut empty) = store();
        let r = empty.import_from(&export, false, None).unwrap();
        assert_eq!((r.groups, r.hosts, r.vms), (1, 1, 1));
        assert_eq!(empty.vms_on_host(&h.id).len(), 1);

        // Remplacement complet
        let r = other.import_from(&export, true, None).unwrap();
        assert_eq!((r.groups, r.hosts, r.vms), (1, 1, 1));
        assert_eq!(other.hosts().len(), 1);
        assert_eq!(other.hosts()[0].id, h.id);
        assert_eq!(other.vms()[0].host_id.as_deref(), Some(h.id.as_str()));
    }

    #[test]
    fn import_fixes_dangling_and_inconsistent_host_refs() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("in.json");
        let mut data = AppData::default();
        let g = Group::from_input(group("Prod"));
        let mut h = Host::from_input(host("pve-01", None));
        h.group_id = Some(g.id.clone());
        let mut lost = Host::from_input(host("orphan-host", None));
        lost.group_id = Some("ghost-group".into());
        let mut dangling = Vm::from_input(vm("dangling", None));
        dangling.host_id = Some("ghost-host".into());
        dangling.group_id = Some(g.id.clone());
        let mut inconsistent = Vm::from_input(vm("inconsistent", None));
        inconsistent.host_id = Some(h.id.clone());
        data.groups.push(g.clone());
        data.hosts.extend([h.clone(), lost.clone()]);
        data.vms.extend([dangling, inconsistent]);
        fs::write(&file, serde_json::to_string(&data).unwrap()).unwrap();

        let (_d, mut s) = store();
        let r = s.import_from(&file, false, None).unwrap();
        assert_eq!((r.groups, r.hosts, r.vms), (1, 2, 2));
        assert_eq!(s.host(&lost.id).unwrap().group_id, None);
        let d = s.vms().iter().find(|v| v.name == "dangling").unwrap();
        assert_eq!(d.host_id, None);
        assert_eq!(d.group_id.as_deref(), Some(g.id.as_str()));
        // La VM placée sur une machine en reprend le groupe
        let i = s.vms().iter().find(|v| v.name == "inconsistent").unwrap();
        assert_eq!(i.host_id.as_deref(), Some(h.id.as_str()));
        assert_eq!(i.group_id.as_deref(), Some(g.id.as_str()));
    }

    #[test]
    fn legacy_json_without_hosts_loads() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");
        fs::write(
            &path,
            r#"{"version":1,"groups":[{"id":"g","name":"Prod","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}],"vms":[{"id":"1","name":"old","group_id":"g","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}]}"#,
        )
        .unwrap();
        let mut s = Store::open(&path).unwrap();
        assert!(s.hosts().is_empty());
        assert_eq!(s.vms()[0].host_id, None);
        assert_eq!(s.vms()[0].group_id.as_deref(), Some("g"));
        // L'inventaire ancien accepte ensuite des machines physiques
        let h = s.create_host(host("pve-01", Some("g"))).unwrap();
        s.set_vm_host("1", Some(&h.id)).unwrap();
        let reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.vms()[0].host_id.as_deref(), Some(h.id.as_str()));
    }

    #[test]
    fn encrypted_store_keeps_hosts() {
        let (dir, mut s, _key, vm_id, _user_id) = store_with_password();
        let path = dir.path().join("data.json");
        s.enable_file_encryption(MASTER).unwrap();
        let h = s.create_host(host("hyperviseur-secret", None)).unwrap();
        s.set_vm_host(&vm_id, Some(&h.id)).unwrap();
        drop(s);

        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("hyperviseur-secret"));
        assert!(matches!(Store::open(&path), Err(CoreError::Locked)));
        let (mut reopened, _key) = Store::open_with_password(&path, MASTER).unwrap();
        assert_eq!(reopened.host(&h.id).unwrap().name, "hyperviseur-secret");
        assert_eq!(reopened.vms_on_host(&h.id).len(), 1);
        reopened
            .update_host(&h.id, host("hyperviseur-renomme", None))
            .unwrap();
        let (again, _key) = Store::open_with_password(&path, MASTER).unwrap();
        assert_eq!(again.hosts()[0].name, "hyperviseur-renomme");
    }
}
