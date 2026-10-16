/// Application egui : état de l'interface et application des modifications.
///
/// Les écrans lisent l'inventaire (`&Store`) et ne le modifient jamais directement :
/// ils déposent des [`Command`] que [`VmManagerApp::run_commands`] applique ensuite,
/// ce qui sépare proprement l'affichage (mode immédiat) des écritures sur le disque.
use std::collections::HashSet;
use std::path::PathBuf;

use vm_core::Store;

use crate::forms::{tab_for_error, GroupForm, UserForm, VmForm};
use crate::paths::DataSource;
use crate::prefs::{self, Prefs};
use crate::theme;
use crate::ui;

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
    pub confirm: Option<Confirm>,
    pub expanded_groups: HashSet<String>,
    pub toasts: Vec<Toast>,
    pub commands: Vec<Command>,
    /// Message de toast en attente (l'horloge egui n'est connue que pendant l'affichage)
    pub pending_toasts: Vec<(bool, String)>,
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
}

pub struct VmManagerApp {
    pub store: Result<Store, String>,
    pub data_path: PathBuf,
    pub data_source: DataSource,
    pub prefs_path: PathBuf,
    pub ctx: egui::Context,
    pub state: UiState,
}

impl VmManagerApp {
    pub fn new(
        store: Result<Store, String>,
        data_path: PathBuf,
        data_source: DataSource,
        prefs_path: PathBuf,
        ctx: egui::Context,
    ) -> Self {
        VmManagerApp {
            store,
            data_path,
            data_source,
            prefs_path,
            ctx,
            state: UiState::default(),
        }
    }

    /// Applique les commandes déposées pendant l'affichage
    pub fn run_commands(&mut self) {
        let Ok(store) = &mut self.store else { return };
        let st = &mut self.state;
        for command in std::mem::take(&mut st.commands) {
            match command {
                Command::SetTheme(index) => {
                    theme::select(&self.ctx, index);
                    let prefs = Prefs {
                        theme: theme::current().id.to_string(),
                    };
                    if let Err(e) = prefs::save(&self.prefs_path, &prefs) {
                        st.error(format!("Thème appliqué mais non enregistré : {e}"));
                    }
                }
                command => run_command(store, st, command),
            }
        }
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
        Command::SaveUser => {
            let Some(form) = &mut st.user_form else {
                return;
            };
            let input = form.to_input();
            let result = match &form.user_id {
                Some(uid) => store.update_user(&form.vm_id, uid, input),
                None => store.add_user(&form.vm_id, input),
            };
            match result {
                Ok(u) => {
                    let created = form.user_id.is_none();
                    st.user_form = None;
                    st.success(if created {
                        format!("Utilisateur « {} » ajouté", u.username)
                    } else {
                        format!("Utilisateur « {} » modifié", u.username)
                    });
                }
                Err(e) => form.error = Some(e.to_string()),
            }
        }
        Command::DeleteUser { vm_id, user_id } => match store.delete_user(&vm_id, &user_id) {
            Ok(()) => st.success("Utilisateur supprimé"),
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
            match store.import_from(&path, replace, None) {
                Ok(report) => {
                    st.open_vm = None;
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
                    st.success(message);
                }
                Err(e) => st.error(e.to_string()),
            }
        }
        Command::SetTheme(_) => {}
    }
}

impl eframe::App for VmManagerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        match &self.store {
            Ok(store) => ui::show(
                ui,
                store,
                &self.data_path,
                self.data_source,
                &mut self.state,
            ),
            Err(err) => ui::load_error(ui, err, &self.data_path),
        }
        self.run_commands();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::VmTab;

    fn app() -> (tempfile::TempDir, Store, UiState) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("d.json")).unwrap();
        (dir, store, UiState::default())
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
}
