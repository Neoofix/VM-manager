/// Application egui : état de l'interface et application des modifications.
///
/// Les écrans lisent l'inventaire (`&Store`) et ne le modifient jamais directement :
/// ils déposent des [`Command`] que [`VmManagerApp::run_commands`] applique ensuite,
/// ce qui sépare proprement l'affichage (mode immédiat) des écritures sur le disque.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use vm_core::{CoreError, CoreResult, ImportReport, KdfParams, Store, VaultKey};

use crate::forms::{
    tab_for_error, GroupForm, PasswordChange, UserForm, VaultForm, VaultMode, VmForm,
};
use crate::paths::DataSource;
use crate::prefs::{self, Prefs};
use crate::theme;
use crate::ui;

/// Inactivité (sans clavier ni souris) au bout de laquelle le coffre se verrouille
pub const AUTO_LOCK_SECS: f64 = 10.0 * 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Dashboard,
    Vms,
    Groups,
    Settings,
}

/// Filtre de groupe de la liste des VM
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GroupFilter {
    #[default]
    All,
    Ungrouped,
    Group(String),
}

/// Modifications de l'inventaire demandées par l'interface
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    SaveVm,
    DuplicateVm(String),
    DeleteVm(String),
    SaveGroup,
    DeleteGroup(String),
    SaveUser,
    DeleteUser {
        vm_id: String,
        user_id: String,
    },
    Export,
    Import {
        replace: bool,
    },
    /// Change de thème (index dans `theme::THEMES`) et l'enregistre
    SetTheme(usize),
    /// Valide la fenêtre du coffre ouverte (`UiState::vault_form`)
    SubmitVault,
    /// Verrouille le coffre immédiatement
    LockVault,
    /// Supprime le coffre et tous les mots de passe enregistrés
    ResetVault,
}

/// Demande de confirmation avant une commande destructive
pub struct Confirm {
    pub title: String,
    pub message: String,
    pub button: &'static str,
    pub command: Command,
}

pub struct Toast {
    pub success: bool,
    pub message: String,
    pub expires_at: f64,
}

#[derive(Default)]
pub struct UiState {
    pub page: Page,
    pub search: String,
    pub group_filter: GroupFilter,
    pub open_vm: Option<String>,
    pub vm_form: Option<VmForm>,
    pub group_form: Option<GroupForm>,
    pub user_form: Option<UserForm>,
    pub vault_form: Option<VaultForm>,
    pub confirm: Option<Confirm>,
    pub expanded_groups: HashSet<String>,
    pub toasts: Vec<Toast>,
    pub commands: Vec<Command>,
    /// Message de toast en attente (l'horloge egui n'est connue que pendant l'affichage)
    pub pending_toasts: Vec<(bool, String)>,
    /// Clé du coffre tant qu'il est déverrouillé : uniquement en mémoire, jamais
    /// écrite ni journalisée, effacée au verrouillage
    pub vault_key: Option<VaultKey>,
    /// Utilisateurs (identifiants) dont le mot de passe est affiché en clair
    pub revealed: HashSet<String>,
    /// Dernière action au clavier ou à la souris (horloge egui, en secondes)
    pub last_activity: f64,
    /// Réglage Argon2id des coffres créés depuis l'interface (rapide dans les tests)
    pub kdf: KdfParams,
}

impl UiState {
    pub fn run(&mut self, command: Command) {
        self.commands.push(command);
    }

    pub fn ask(&mut self, title: impl Into<String>, message: impl Into<String>, command: Command) {
        self.confirm = Some(Confirm {
            title: title.into(),
            message: message.into(),
            button: "Supprimer",
            command,
        });
    }

    pub fn success(&mut self, message: impl Into<String>) {
        self.pending_toasts.push((true, message.into()));
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.pending_toasts.push((false, message.into()));
    }

    pub fn open_vms(&mut self, filter: GroupFilter) {
        self.page = Page::Vms;
        self.group_filter = filter;
        self.open_vm = None;
    }

    pub fn open_vm(&mut self, id: &str) {
        self.page = Page::Vms;
        self.open_vm = Some(id.to_string());
    }

    pub fn is_unlocked(&self) -> bool {
        self.vault_key.is_some()
    }

    /// Oublie la clé du coffre et masque les mots de passe affichés
    pub fn lock(&mut self) {
        self.vault_key = None;
        self.revealed.clear();
    }

    /// Ouvre la fenêtre de déverrouillage (ou de création s'il n'y a pas encore de coffre)
    pub fn ask_unlock(&mut self, vault_exists: bool, then: Option<Command>) {
        let mut form = VaultForm::unlock_or_create(vault_exists);
        form.then = then.map(Box::new);
        self.vault_form = Some(form);
    }

    /// Note l'activité de l'utilisateur et verrouille le coffre après
    /// [`AUTO_LOCK_SECS`] d'inactivité. Renvoie le délai restant avant le
    /// verrouillage automatique (coffre déverrouillé), pour réveiller l'interface à temps.
    pub fn tick(&mut self, now: f64, active: bool) -> Option<f64> {
        if active {
            self.last_activity = now;
        }
        self.vault_key.as_ref()?;
        let idle = now - self.last_activity;
        if idle >= AUTO_LOCK_SECS {
            self.lock();
            self.success("Coffre verrouillé automatiquement après 10 minutes d'inactivité");
            None
        } else {
            Some(AUTO_LOCK_SECS - idle)
        }
    }
}

/// État du fichier de données au lancement
pub enum StoreState {
    Loaded(Box<Store>),
    /// Fichier entièrement chiffré : mot de passe maître attendu (écran de déverrouillage)
    Locked,
    /// Fichier illisible ou inaccessible
    Error(String),
}

impl StoreState {
    pub fn from_open(result: CoreResult<Store>) -> Self {
        match result {
            Ok(store) => StoreState::Loaded(Box::new(store)),
            Err(CoreError::Locked) => StoreState::Locked,
            Err(e) => StoreState::Error(e.to_string()),
        }
    }
}

pub struct VmManagerApp {
    pub store: StoreState,
    pub data_path: PathBuf,
    pub data_source: DataSource,
    pub prefs_path: PathBuf,
    pub ctx: egui::Context,
    pub state: UiState,
}

impl VmManagerApp {
    pub fn new(
        store: StoreState,
        data_path: PathBuf,
        data_source: DataSource,
        prefs_path: PathBuf,
        ctx: egui::Context,
    ) -> Self {
        let mut state = UiState::default();
        if matches!(store, StoreState::Locked) {
            state.vault_form = Some(VaultForm::new(VaultMode::OpenFile));
        }
        VmManagerApp {
            store,
            data_path,
            data_source,
            prefs_path,
            ctx,
            state,
        }
    }

    /// Applique les commandes déposées pendant l'affichage
    pub fn run_commands(&mut self) {
        let st = &mut self.state;
        for command in std::mem::take(&mut st.commands) {
            if let Command::SetTheme(index) = command {
                theme::select(&self.ctx, index);
                let prefs = Prefs {
                    theme: theme::current().id.to_string(),
                };
                if let Err(e) = prefs::save(&self.prefs_path, &prefs) {
                    st.error(format!("Thème appliqué mais non enregistré : {e}"));
                }
                continue;
            }
            let opened = match &mut self.store {
                StoreState::Loaded(store) => {
                    run_command(store, st, command);
                    None
                }
                StoreState::Locked if command == Command::SubmitVault => {
                    open_encrypted(&self.data_path, st)
                }
                _ => None,
            };
            if let Some(store) = opened {
                self.store = StoreState::Loaded(Box::new(store));
            }
        }
    }
}

/// Écran de déverrouillage : ouvre l'inventaire chiffré avec le mot de passe saisi
fn open_encrypted(path: &Path, st: &mut UiState) -> Option<Store> {
    let form = st.vault_form.as_mut()?;
    if let Err(e) = form.check() {
        form.error = Some(e);
        return None;
    }
    match Store::open_with_password(path, &form.password) {
        Ok((store, key)) => {
            st.vault_form = None;
            st.vault_key = Some(key);
            st.success("Inventaire déverrouillé");
            Some(store)
        }
        Err(e) => {
            form.password.clear();
            form.focused = false;
            form.error = Some(e.to_string());
            None
        }
    }
}

fn import_message(report: ImportReport) -> String {
    let mut message = format!(
        "Import terminé : {} groupe(s) et {} VM ajouté(s)",
        report.groups, report.vms
    );
    if report.passwords_dropped > 0 {
        message.push_str(&format!(
            " — {} mot(s) de passe écarté(s) (coffre différent)",
            report.passwords_dropped
        ));
    }
    message
}

/// Oublie une clé qui n'ouvre plus le coffre (coffre remplacé par un import, supprimé…)
fn drop_stale_key(store: &Store, st: &mut UiState) {
    let valid = match (&st.vault_key, store.vault()) {
        (Some(key), Some(header)) => header.verify(key),
        (Some(_), None) => false,
        (None, _) => true,
    };
    if !valid {
        st.lock();
    }
}

fn run_command(store: &mut Store, st: &mut UiState, command: Command) {
    match command {
        Command::SaveVm => {
            let Some(form) = &mut st.vm_form else { return };
            let input = match form.to_input() {
                Ok(input) => input,
                Err((tab, msg)) => {
                    form.tab = tab;
                    form.error = Some(msg);
                    return;
                }
            };
            let result = match &form.edit_id {
                Some(id) => store.update_vm(id, input),
                None => store.create_vm(input),
            };
            match result {
                Ok(vm) => {
                    let created = form.edit_id.is_none();
                    st.vm_form = None;
                    st.open_vm = Some(vm.id.clone());
                    st.success(if created {
                        format!("VM « {} » créée", vm.name)
                    } else {
                        format!("VM « {} » mise à jour", vm.name)
                    });
                }
                Err(e) => {
                    let msg = e.to_string();
                    if let Some(tab) = tab_for_error(&msg) {
                        form.tab = tab;
                    }
                    form.error = Some(msg);
                }
            }
        }
        Command::DuplicateVm(id) => match store.duplicate_vm(&id) {
            Ok(copy) => {
                st.open_vm = Some(copy.id.clone());
                st.success(format!("Copie créée : « {} »", copy.name));
            }
            Err(e) => st.error(e.to_string()),
        },
        Command::DeleteVm(id) => match store.delete_vm(&id) {
            Ok(()) => {
                st.open_vm = None;
                st.success("VM supprimée");
            }
            Err(e) => st.error(e.to_string()),
        },
        Command::SaveGroup => {
            let Some(form) = &mut st.group_form else {
                return;
            };
            let result = match &form.edit_id {
                Some(id) => store.update_group(id, form.to_input()),
                None => store.create_group(form.to_input()),
            };
            match result {
                Ok(g) => {
                    let created = form.edit_id.is_none();
                    st.group_form = None;
                    st.success(if created {
                        format!("Groupe « {} » créé", g.name)
                    } else {
                        format!("Groupe « {} » mis à jour", g.name)
                    });
                }
                Err(e) => form.error = Some(e.to_string()),
            }
        }
        Command::DeleteGroup(id) => match store.delete_group(&id) {
            Ok(detached) => {
                if st.group_filter == GroupFilter::Group(id) {
                    st.group_filter = GroupFilter::All;
                }
                st.success(if detached > 0 {
                    format!("Groupe supprimé, {detached} VM désormais non classée(s)")
                } else {
                    "Groupe supprimé".to_string()
                });
            }
            Err(e) => st.error(e.to_string()),
        },
        Command::SaveUser => save_user(store, st),
        Command::DeleteUser { vm_id, user_id } => match store.delete_user(&vm_id, &user_id) {
            Ok(()) => {
                st.revealed.remove(&user_id);
                st.success("Utilisateur supprimé");
            }
            Err(e) => st.error(e.to_string()),
        },
        Command::Export => {
            let name = format!(
                "vm-manager-{}.json",
                chrono::Local::now().format("%Y-%m-%d")
            );
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Inventaire JSON", &["json"])
                .set_file_name(name)
                .save_file()
            else {
                return;
            };
            match store.export_to(&path) {
                Ok(()) => st.success(format!("Inventaire exporté vers {}", path.display())),
                Err(e) => st.error(e.to_string()),
            }
        }
        Command::Import { replace } => {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("Inventaire JSON", &["json"])
                .pick_file()
            else {
                return;
            };
            import(store, st, path, replace);
        }
        Command::SubmitVault => submit_vault(store, st),
        Command::LockVault => {
            st.lock();
            st.success("Coffre verrouillé");
        }
        Command::ResetVault => match store.reset_vault() {
            Ok(removed) => {
                st.lock();
                st.success(format!(
                    "Coffre réinitialisé : {removed} mot(s) de passe supprimé(s)"
                ));
            }
            Err(e) => st.error(e.to_string()),
        },
        Command::SetTheme(_) => {}
    }
}

/// Import d'un fichier ; un fichier chiffré demande d'abord son mot de passe maître
fn import(store: &mut Store, st: &mut UiState, path: PathBuf, replace: bool) {
    match store.import_from(&path, replace, None) {
        Ok(report) => {
            st.open_vm = None;
            drop_stale_key(store, st);
            st.success(import_message(report));
        }
        Err(CoreError::Locked) => {
            st.vault_form = Some(VaultForm::new(VaultMode::Import { path, replace }));
        }
        Err(e) => st.error(e.to_string()),
    }
}

/// Enregistre l'utilisateur, puis son mot de passe (chiffré par le coffre) s'il a changé
fn save_user(store: &mut Store, st: &mut UiState) {
    let Some(form) = &mut st.user_form else {
        return;
    };
    let change = form.password_change();
    if change != PasswordChange::Keep && st.vault_key.is_none() {
        // Le formulaire reste ouvert : l'enregistrement reprend après le déverrouillage
        let exists = store.vault().is_some();
        form.error = Some(if exists {
            "Déverrouillez le coffre pour enregistrer le mot de passe".into()
        } else {
            "Créez d'abord le coffre des mots de passe (mot de passe maître)".into()
        });
        st.ask_unlock(exists, Some(Command::SaveUser));
        return;
    }

    let input = form.to_input();
    let result = match &form.user_id {
        Some(uid) => store.update_user(&form.vm_id, uid, input),
        None => store.add_user(&form.vm_id, input),
    };
    let user = match result {
        Ok(u) => u,
        Err(e) => {
            form.error = Some(e.to_string());
            return;
        }
    };
    let saved = match (change, &st.vault_key) {
        (PasswordChange::Set(pw), Some(key)) => {
            store.set_user_password(key, &form.vm_id, &user.id, Some(pw))
        }
        (PasswordChange::Remove, Some(key)) => {
            store.set_user_password(key, &form.vm_id, &user.id, None)
        }
        _ => Ok(()),
    };
    if let Err(e) = saved {
        // L'utilisateur existe désormais : le formulaire passe en modification
        form.user_id = Some(user.id.clone());
        form.error = Some(format!(
            "Utilisateur enregistré, mais pas son mot de passe : {e}"
        ));
        return;
    }
    let created = form.user_id.is_none();
    st.user_form = None;
    st.success(if created {
        format!("Utilisateur « {} » ajouté", user.username)
    } else {
        format!("Utilisateur « {} » modifié", user.username)
    });
}

/// Valide la fenêtre du coffre selon son mode
fn submit_vault(store: &mut Store, st: &mut UiState) {
    let Some(form) = &mut st.vault_form else {
        return;
    };
    if let Err(e) = form.check() {
        form.error = Some(e);
        return;
    }
    let pw = form.password.as_str();
    let result: CoreResult<(Option<VaultKey>, String)> = match &form.mode {
        VaultMode::Create => store.create_vault_with(pw, st.kdf).map(|key| {
            (
                Some(key),
                "Coffre créé — conserve précieusement le mot de passe maître".into(),
            )
        }),
        VaultMode::Unlock => store
            .unlock_vault(pw)
            .map(|key| (Some(key), "Coffre déverrouillé".into())),
        VaultMode::ChangeMaster => store
            .change_master_password(pw, &form.new_password)
            .map(|key| (Some(key), "Mot de passe maître changé".into())),
        VaultMode::EnableFileEncryption => store
            .enable_file_encryption_with(pw, st.kdf)
            .map(|key| (Some(key), "Fichier de données chiffré".into())),
        VaultMode::DisableFileEncryption => store
            .disable_file_encryption(pw)
            .map(|()| (None, "Fichier de données enregistré en clair".into())),
        VaultMode::Import { path, replace } => store
            .import_from(path, *replace, Some(pw))
            .map(|report| (None, import_message(report))),
        // Écran de déverrouillage du démarrage : géré par `VmManagerApp`
        VaultMode::OpenFile => return,
    };
    match result {
        Ok((key, message)) => {
            let form = st.vault_form.take().expect("fenêtre du coffre ouverte");
            if let Some(key) = key {
                st.vault_key = Some(key);
            }
            if matches!(form.mode, VaultMode::Import { .. }) {
                st.open_vm = None;
                drop_stale_key(store, st);
            }
            st.success(message);
            if let Some(then) = form.then {
                run_command(store, st, *then);
            }
        }
        Err(e) => {
            form.error = Some(e.to_string());
            form.focused = false;
        }
    }
}

impl eframe::App for VmManagerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let (now, active) = ui.input(|i| (i.time, !i.events.is_empty()));
        if let Some(remaining) = self.state.tick(now, active) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(remaining + 0.5));
        }
        match &self.store {
            StoreState::Loaded(store) => ui::show(
                ui,
                store,
                &self.data_path,
                self.data_source,
                &mut self.state,
            ),
            StoreState::Locked => ui::unlock_screen(ui, &self.data_path, &mut self.state),
            StoreState::Error(err) => ui::load_error(ui, err, &self.data_path),
        }
        self.run_commands();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::VmTab;
    use zeroize::Zeroizing;

    const MASTER: &str = "maitre-de-test";

    fn app() -> (tempfile::TempDir, Store, UiState) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("d.json")).unwrap();
        let st = UiState {
            kdf: KdfParams::insecure_for_tests(),
            ..Default::default()
        };
        (dir, store, st)
    }

    /// Inventaire avec coffre déverrouillé et une VM ; renvoie l'identifiant de la VM
    fn unlocked() -> (tempfile::TempDir, Store, UiState, String) {
        let (dir, mut store, mut st) = app();
        st.vault_key = Some(
            store
                .create_vault_with(MASTER, KdfParams::insecure_for_tests())
                .unwrap(),
        );
        let vm = store
            .create_vm(vm_core::VmInput {
                name: "web".into(),
                ..Default::default()
            })
            .unwrap();
        (dir, store, st, vm.id)
    }

    fn secret(s: &str) -> Zeroizing<String> {
        Zeroizing::new(s.into())
    }

    fn stored_password(store: &Store, st: &UiState, vm_id: &str) -> Option<String> {
        let user = &store.vm(vm_id).unwrap().users[0];
        store
            .user_password(st.vault_key.as_ref().unwrap(), vm_id, &user.id)
            .unwrap()
            .map(|p| p.to_string())
    }

    #[test]
    fn saving_a_new_vm_opens_it() {
        let (_d, mut store, mut st) = app();
        let mut form = VmForm::create(None);
        form.name = "web".into();
        st.vm_form = Some(form);
        run_command(&mut store, &mut st, Command::SaveVm);
        assert!(st.vm_form.is_none());
        assert_eq!(st.open_vm.as_deref(), Some(store.vms()[0].id.as_str()));
        assert_eq!(st.pending_toasts.len(), 1);
    }

    #[test]
    fn invalid_vm_keeps_form_open_on_faulty_tab() {
        let (_d, mut store, mut st) = app();
        let mut form = VmForm::create(None);
        form.name = "web".into();
        form.gateway = "pas-une-ip".into();
        st.vm_form = Some(form);
        run_command(&mut store, &mut st, Command::SaveVm);
        let form = st.vm_form.as_ref().unwrap();
        assert_eq!(form.tab, VmTab::Network);
        assert!(form.error.as_ref().unwrap().contains("Passerelle"));
        assert!(store.vms().is_empty());
    }

    #[test]
    fn users_and_groups_flow() {
        let (_d, mut store, mut st) = app();
        let mut g = GroupForm::create();
        g.name = "Prod".into();
        st.group_form = Some(g);
        run_command(&mut store, &mut st, Command::SaveGroup);
        let gid = store.groups()[0].id.clone();

        let vm = store
            .create_vm(vm_core::VmInput {
                name: "a".into(),
                group_id: Some(gid.clone()),
                ..Default::default()
            })
            .unwrap();
        let mut u = UserForm::create(&vm.id);
        u.username = "root".into();
        st.user_form = Some(u.clone());
        run_command(&mut store, &mut st, Command::SaveUser);
        assert!(st.user_form.is_none());

        // Doublon : le formulaire reste ouvert avec l'erreur
        st.user_form = Some(u);
        run_command(&mut store, &mut st, Command::SaveUser);
        assert!(st.user_form.as_ref().unwrap().error.is_some());

        st.group_filter = GroupFilter::Group(gid.clone());
        run_command(&mut store, &mut st, Command::DeleteGroup(gid));
        assert_eq!(st.group_filter, GroupFilter::All);
        assert_eq!(store.vms()[0].group_id, None);
    }

    #[test]
    fn user_password_is_stored_encrypted() {
        let (dir, mut store, mut st, vm_id) = unlocked();
        let mut u = UserForm::create(&vm_id);
        u.username = "root".into();
        u.password = secret("Tr3s-s3cret!");
        st.user_form = Some(u);
        run_command(&mut store, &mut st, Command::SaveUser);
        assert!(st.user_form.is_none());
        assert!(store.vm(&vm_id).unwrap().users[0].password.is_some());
        assert_eq!(
            stored_password(&store, &st, &vm_id).as_deref(),
            Some("Tr3s-s3cret!")
        );
        let raw = std::fs::read_to_string(dir.path().join("d.json")).unwrap();
        assert!(!raw.contains("Tr3s-s3cret!"));
    }

    #[test]
    fn empty_password_on_edit_keeps_it_and_remove_deletes_it() {
        let (_d, mut store, mut st, vm_id) = unlocked();
        let mut u = UserForm::create(&vm_id);
        u.username = "root".into();
        u.password = secret("premier");
        st.user_form = Some(u);
        run_command(&mut store, &mut st, Command::SaveUser);

        let user = store.vm(&vm_id).unwrap().users[0].clone();
        let mut edit = UserForm::edit(&vm_id, &user);
        assert!(edit.has_password && edit.password.is_empty());
        edit.role = "admin".into();
        st.user_form = Some(edit);
        run_command(&mut store, &mut st, Command::SaveUser);
        assert!(st.user_form.is_none());
        assert_eq!(store.vm(&vm_id).unwrap().users[0].role, "admin");
        assert_eq!(
            stored_password(&store, &st, &vm_id).as_deref(),
            Some("premier")
        );

        // Nouveau mot de passe saisi : remplacé
        let user = store.vm(&vm_id).unwrap().users[0].clone();
        let mut edit = UserForm::edit(&vm_id, &user);
        edit.password = secret("second");
        st.user_form = Some(edit);
        run_command(&mut store, &mut st, Command::SaveUser);
        assert_eq!(
            stored_password(&store, &st, &vm_id).as_deref(),
            Some("second")
        );

        let user = store.vm(&vm_id).unwrap().users[0].clone();
        let mut edit = UserForm::edit(&vm_id, &user);
        edit.remove_password = true;
        st.user_form = Some(edit);
        run_command(&mut store, &mut st, Command::SaveUser);
        assert!(store.vm(&vm_id).unwrap().users[0].password.is_none());
    }

    #[test]
    fn saving_a_password_while_locked_asks_for_the_vault() {
        let (_d, mut store, mut st, vm_id) = unlocked();
        st.lock();
        let mut u = UserForm::create(&vm_id);
        u.username = "root".into();
        u.password = secret("hunter2");
        st.user_form = Some(u);
        run_command(&mut store, &mut st, Command::SaveUser);

        // Rien d'enregistré, formulaire ouvert, déverrouillage demandé
        assert!(store.vm(&vm_id).unwrap().users.is_empty());
        assert!(st.user_form.as_ref().unwrap().error.is_some());
        let vault = st.vault_form.as_mut().unwrap();
        assert_eq!(vault.mode, VaultMode::Unlock);
        assert_eq!(vault.then.as_deref(), Some(&Command::SaveUser));

        // Mauvais mot de passe maître : la fenêtre reste ouverte
        vault.password = secret("pas-le-bon");
        run_command(&mut store, &mut st, Command::SubmitVault);
        assert!(st.vault_form.as_ref().unwrap().error.is_some());
        assert!(!st.is_unlocked());

        // Bon mot de passe : déverrouillage puis enregistrement repris
        st.vault_form.as_mut().unwrap().password = secret(MASTER);
        run_command(&mut store, &mut st, Command::SubmitVault);
        assert!(st.vault_form.is_none());
        assert!(st.is_unlocked());
        assert!(st.user_form.is_none());
        assert_eq!(
            stored_password(&store, &st, &vm_id).as_deref(),
            Some("hunter2")
        );
    }

    #[test]
    fn saving_a_password_without_vault_offers_to_create_it() {
        let (_d, mut store, mut st) = app();
        let vm = store
            .create_vm(vm_core::VmInput {
                name: "web".into(),
                ..Default::default()
            })
            .unwrap();
        let mut u = UserForm::create(&vm.id);
        u.username = "root".into();
        u.password = secret("hunter2");
        st.user_form = Some(u);
        run_command(&mut store, &mut st, Command::SaveUser);
        let vault = st.vault_form.as_mut().unwrap();
        assert_eq!(vault.mode, VaultMode::Create);

        vault.password = secret(MASTER);
        vault.confirm = secret("autre-chose");
        run_command(&mut store, &mut st, Command::SubmitVault);
        assert!(store.vault().is_none());
        assert!(st.vault_form.as_ref().unwrap().error.is_some());

        st.vault_form.as_mut().unwrap().confirm = secret(MASTER);
        run_command(&mut store, &mut st, Command::SubmitVault);
        assert!(store.vault().is_some());
        assert!(st.user_form.is_none());
        assert_eq!(
            stored_password(&store, &st, &vm.id).as_deref(),
            Some("hunter2")
        );
    }

    #[test]
    fn file_encryption_commands() {
        let (dir, mut store, mut st, _vm) = unlocked();
        let path = dir.path().join("d.json");
        st.lock();

        st.vault_form = Some(VaultForm::new(VaultMode::EnableFileEncryption));
        st.vault_form.as_mut().unwrap().password = secret("pas-le-bon");
        run_command(&mut store, &mut st, Command::SubmitVault);
        assert!(!store.is_file_encrypted());
        assert!(st.vault_form.as_ref().unwrap().error.is_some());

        st.vault_form.as_mut().unwrap().password = secret(MASTER);
        run_command(&mut store, &mut st, Command::SubmitVault);
        assert!(store.is_file_encrypted());
        assert!(st.is_unlocked());
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("web"));
        assert!(matches!(
            StoreState::from_open(Store::open(&path)),
            StoreState::Locked
        ));

        // Réinitialisation refusée tant que le fichier est chiffré
        run_command(&mut store, &mut st, Command::ResetVault);
        assert!(store.vault().is_some());
        assert!(st.is_unlocked());

        st.vault_form = Some(VaultForm::new(VaultMode::DisableFileEncryption));
        st.vault_form.as_mut().unwrap().password = secret(MASTER);
        run_command(&mut store, &mut st, Command::SubmitVault);
        assert!(st.vault_form.is_none());
        assert!(!store.is_file_encrypted());
        assert!(std::fs::read_to_string(&path).unwrap().contains("web"));

        run_command(&mut store, &mut st, Command::ResetVault);
        assert!(store.vault().is_none());
        assert!(!st.is_unlocked());
    }

    #[test]
    fn unlock_screen_opens_encrypted_file() {
        let (dir, mut store, _st, _vm) = unlocked();
        store.enable_file_encryption(MASTER).unwrap();
        let path = dir.path().join("d.json");
        drop(store);

        let mut st = UiState {
            vault_form: Some(VaultForm::new(VaultMode::OpenFile)),
            ..Default::default()
        };
        st.vault_form.as_mut().unwrap().password = secret("pas-le-bon");
        assert!(open_encrypted(&path, &mut st).is_none());
        let form = st.vault_form.as_ref().unwrap();
        assert_eq!(form.error.as_deref(), Some("Mot de passe maître incorrect"));
        assert!(form.password.is_empty());

        st.vault_form.as_mut().unwrap().password = secret(MASTER);
        let store = open_encrypted(&path, &mut st).expect("inventaire ouvert");
        assert_eq!(store.vms()[0].name, "web");
        assert!(st.is_unlocked() && st.vault_form.is_none());
    }

    #[test]
    fn change_master_password_command() {
        let (_d, mut store, mut st, _vm) = unlocked();
        let mut form = VaultForm::new(VaultMode::ChangeMaster);
        form.password = secret(MASTER);
        form.new_password = secret("nouveau-maitre");
        form.confirm = secret("nouveau-maitre");
        st.vault_form = Some(form);
        run_command(&mut store, &mut st, Command::SubmitVault);
        assert!(st.vault_form.is_none());
        assert!(store.unlock_vault("nouveau-maitre").is_ok());
        let header = store.vault().unwrap();
        assert!(header.verify(st.vault_key.as_ref().unwrap()));
    }

    #[test]
    fn auto_lock_after_inactivity() {
        let (_d, _store, mut st, _vm) = unlocked();
        st.revealed.insert("u".into());
        assert_eq!(st.tick(5.0, true), Some(AUTO_LOCK_SECS));
        assert!(st.tick(5.0 + AUTO_LOCK_SECS - 1.0, false).is_some());
        // Une action repousse l'échéance
        assert_eq!(st.tick(100.0, true), Some(AUTO_LOCK_SECS));
        assert!(st.tick(100.0 + AUTO_LOCK_SECS - 1.0, false).is_some());
        assert!(st.is_unlocked());
        assert_eq!(st.tick(100.0 + AUTO_LOCK_SECS, false), None);
        assert!(!st.is_unlocked());
        assert!(st.revealed.is_empty());
        // Coffre verrouillé : plus de délai à surveiller
        assert_eq!(st.tick(10_000.0, true), None);
    }

    #[test]
    fn lock_command_forgets_key() {
        let (_d, mut store, mut st, _vm) = unlocked();
        st.revealed.insert("u".into());
        run_command(&mut store, &mut st, Command::LockVault);
        assert!(!st.is_unlocked());
        assert!(st.revealed.is_empty());
    }

    #[test]
    fn encrypted_import_asks_for_password() {
        let (dir, mut store, _st, _vm) = unlocked();
        store.enable_file_encryption(MASTER).unwrap();
        let export = dir.path().join("export.json");
        store.export_to(&export).unwrap();

        let (_d2, mut other, mut st2) = app();
        import(&mut other, &mut st2, export.clone(), false);
        let form = st2.vault_form.as_mut().expect("mot de passe demandé");
        assert_eq!(
            form.mode,
            VaultMode::Import {
                path: export,
                replace: false
            }
        );
        form.password = secret("pas-le-bon");
        run_command(&mut other, &mut st2, Command::SubmitVault);
        assert!(other.vms().is_empty());
        assert!(st2.vault_form.as_ref().unwrap().error.is_some());

        st2.vault_form.as_mut().unwrap().password = secret(MASTER);
        run_command(&mut other, &mut st2, Command::SubmitVault);
        assert!(st2.vault_form.is_none());
        assert_eq!(other.vms()[0].name, "web");
        assert!(st2.pending_toasts.last().unwrap().1.contains("1 VM"));
    }
}
