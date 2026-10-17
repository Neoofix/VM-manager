/// États des formulaires : champs texte librement éditables, convertis en
/// saisies `vm-core` à l'enregistrement (la validation métier reste dans le cœur)
use std::path::PathBuf;

use vm_core::vault::validate_master_password;
use vm_core::{
    Group, GroupInput, Host, NetworkInfo, SshAuthMethod, SshInfo, Vm, VmInput, VmUser, VmUserInput,
    DEFAULT_SSH_PORT,
};

use zeroize::Zeroizing;

use crate::app::Command;
use crate::theme::GROUP_COLORS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VmTab {
    #[default]
    General,
    Network,
    Ssh,
}

#[derive(Debug, Clone, Default)]
pub struct VmForm {
    /// `None` = création
    pub edit_id: Option<String>,
    pub tab: VmTab,
    pub error: Option<String>,
    /// Le premier champ a déjà reçu le focus clavier
    pub focused: bool,

    pub name: String,
    pub description: String,
    pub group_id: Option<String>,
    /// Machine physique ; son groupe s'impose à la VM à l'enregistrement
    pub host_id: Option<String>,
    pub os: String,
    pub tags: String,
    pub notes: String,

    pub ip: String,
    pub hostname: String,
    pub subnet: String,
    pub gateway: String,
    pub vlan: String,
    pub dns: String,
    pub mac: String,
    pub bridge: String,

    pub port: String,
    pub default_user: String,
    pub auth_method: SshAuthMethod,
    pub key_reference: String,
    pub ssh_notes: String,
}

impl VmForm {
    pub fn create(group_id: Option<String>) -> Self {
        VmForm {
            group_id,
            port: DEFAULT_SSH_PORT.to_string(),
            ..Default::default()
        }
    }

    /// Création préréglée sur une machine physique (dont le groupe s'impose)
    pub fn create_on(group_id: Option<String>, host: Option<&Host>) -> Self {
        let mut form = Self::create(group_id);
        if host.is_some() {
            form.set_host(host);
        }
        form
    }

    /// Choisit la machine physique : la VM prend alors le groupe de la machine.
    /// « Aucune » (`None`) laisse le groupe actuel, de nouveau modifiable.
    pub fn set_host(&mut self, host: Option<&Host>) {
        self.host_id = host.map(|h| h.id.clone());
        if let Some(h) = host {
            self.group_id = h.group_id.clone();
        }
    }

    /// Réaligne le formulaire sur l'inventaire : machine disparue oubliée,
    /// groupe recopié depuis la machine (règle appliquée aussi par le cœur)
    pub fn sync_host(&mut self, hosts: &[Host]) {
        let Some(id) = &self.host_id else { return };
        match hosts.iter().find(|h| &h.id == id) {
            Some(h) => self.group_id = h.group_id.clone(),
            None => self.host_id = None,
        }
    }

    pub fn edit(vm: &Vm) -> Self {
        let n = &vm.network;
        VmForm {
            edit_id: Some(vm.id.clone()),
            tab: VmTab::General,
            error: None,
            focused: false,
            name: vm.name.clone(),
            description: vm.description.clone(),
            group_id: vm.group_id.clone(),
            host_id: vm.host_id.clone(),
            os: vm.os.clone(),
            tags: vm.tags.join(", "),
            notes: vm.notes.clone(),
            ip: n.ip.clone(),
            hostname: n.hostname.clone(),
            subnet: n.subnet.clone(),
            gateway: n.gateway.clone(),
            vlan: n.vlan.map(|v| v.to_string()).unwrap_or_default(),
            dns: n.dns.join(", "),
            mac: n.mac.clone(),
            bridge: n.bridge.clone(),
            port: vm.ssh.port.to_string(),
            default_user: vm.ssh.default_user.clone(),
            auth_method: vm.ssh.auth_method,
            key_reference: vm.ssh.key_reference.clone(),
            ssh_notes: vm.ssh.notes.clone(),
        }
    }

    /// Convertit les champs numériques ; renvoie l'onglet fautif avec l'erreur
    pub fn to_input(&self) -> Result<VmInput, (VmTab, String)> {
        let vlan = match self.vlan.trim() {
            "" => None,
            v => Some(v.parse::<u16>().map_err(|_| {
                (
                    VmTab::Network,
                    format!("VLAN invalide : « {v} » (attendu entre 1 et 4094)"),
                )
            })?),
        };
        let port = match self.port.trim() {
            "" => DEFAULT_SSH_PORT,
            p => p.parse::<u16>().map_err(|_| {
                (
                    VmTab::Ssh,
                    format!("Port SSH invalide : « {p} » (attendu entre 1 et 65535)"),
                )
            })?,
        };
        Ok(VmInput {
            name: self.name.clone(),
            description: self.description.clone(),
            group_id: self.group_id.clone(),
            host_id: self.host_id.clone(),
            os: self.os.clone(),
            tags: split_list(&self.tags),
            notes: self.notes.clone(),
            network: NetworkInfo {
                ip: self.ip.clone(),
                hostname: self.hostname.clone(),
                vlan,
                subnet: self.subnet.clone(),
                gateway: self.gateway.clone(),
                dns: split_list(&self.dns),
                mac: self.mac.clone(),
                bridge: self.bridge.clone(),
            },
            ssh: SshInfo {
                port,
                default_user: self.default_user.clone(),
                auth_method: self.auth_method,
                key_reference: self.key_reference.clone(),
                notes: self.ssh_notes.clone(),
            },
        })
    }
}

/// Onglet où se trouve probablement le champ mentionné par une erreur du cœur
pub fn tab_for_error(message: &str) -> Option<VmTab> {
    let m = message.to_lowercase();
    if [
        "ip",
        "passerelle",
        "dns",
        "vlan",
        "mac",
        "sous-réseau",
        "hôte",
    ]
    .iter()
    .any(|k| m.contains(k))
    {
        Some(VmTab::Network)
    } else if m.contains("ssh") || m.contains("utilisateur") {
        Some(VmTab::Ssh)
    } else if m.contains("nom") || m.contains("groupe") {
        Some(VmTab::General)
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct GroupForm {
    pub edit_id: Option<String>,
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub error: Option<String>,
    pub focused: bool,
}

impl GroupForm {
    pub fn create() -> Self {
        GroupForm {
            edit_id: None,
            name: String::new(),
            description: String::new(),
            color: Some(GROUP_COLORS[0].to_string()),
            error: None,
            focused: false,
        }
    }

    pub fn edit(g: &Group) -> Self {
        GroupForm {
            edit_id: Some(g.id.clone()),
            name: g.name.clone(),
            description: g.description.clone(),
            color: g.color.clone(),
            error: None,
            focused: false,
        }
    }

    pub fn to_input(&self) -> GroupInput {
        GroupInput {
            name: self.name.clone(),
            description: self.description.clone(),
            color: self.color.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserForm {
    pub vm_id: String,
    /// `None` = ajout
    pub user_id: Option<String>,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub sudo: bool,
    pub notes: String,
    /// Nouveau mot de passe ; vide = ne rien changer (effacé de la mémoire à la fermeture)
    pub password: Zeroizing<String>,
    /// Affiche le mot de passe saisi en clair
    pub show_password: bool,
    /// L'utilisateur modifié a déjà un mot de passe enregistré
    pub has_password: bool,
    /// Supprimer le mot de passe enregistré
    pub remove_password: bool,
    pub error: Option<String>,
    pub focused: bool,
}

/// Changement de mot de passe demandé par le formulaire d'utilisateur
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordChange<'a> {
    Keep,
    Set(&'a str),
    Remove,
}

impl UserForm {
    pub fn create(vm_id: &str) -> Self {
        UserForm {
            vm_id: vm_id.to_string(),
            ..Default::default()
        }
    }

    pub fn edit(vm_id: &str, u: &VmUser) -> Self {
        UserForm {
            vm_id: vm_id.to_string(),
            user_id: Some(u.id.clone()),
            username: u.username.clone(),
            full_name: u.full_name.clone(),
            role: u.role.clone(),
            sudo: u.sudo,
            notes: u.notes.clone(),
            has_password: u.password.is_some(),
            ..Default::default()
        }
    }

    pub fn to_input(&self) -> VmUserInput {
        VmUserInput {
            username: self.username.clone(),
            full_name: self.full_name.clone(),
            role: self.role.clone(),
            sudo: self.sudo,
            notes: self.notes.clone(),
        }
    }

    /// Ce qu'il faut faire du mot de passe à l'enregistrement
    pub fn password_change(&self) -> PasswordChange<'_> {
        if self.remove_password && self.has_password {
            PasswordChange::Remove
        } else if self.password.is_empty() {
            PasswordChange::Keep
        } else {
            PasswordChange::Set(self.password.as_str())
        }
    }
}

// ── Coffre des mots de passe ──────────────────────────────────────────────

/// Action demandée par la fenêtre du coffre
#[derive(Debug, Clone, PartialEq)]
pub enum VaultMode {
    /// Création du coffre (mot de passe + confirmation)
    Create,
    Unlock,
    /// Ancien, nouveau et confirmation
    ChangeMaster,
    EnableFileEncryption,
    DisableFileEncryption,
    /// Ouverture au démarrage d'un inventaire entièrement chiffré
    OpenFile,
    /// Import d'un fichier exporté chiffré : mot de passe maître de ce fichier
    Import {
        path: PathBuf,
        replace: bool,
    },
}

/// Fenêtre de saisie du mot de passe maître
#[derive(Debug, Clone)]
pub struct VaultForm {
    pub mode: VaultMode,
    /// Mot de passe maître (actuel, ou nouveau à la création)
    pub password: Zeroizing<String>,
    /// Nouveau mot de passe maître (changement)
    pub new_password: Zeroizing<String>,
    pub confirm: Zeroizing<String>,
    /// Affiche les saisies en clair
    pub reveal: bool,
    /// Chiffrement du fichier sans coffre existant : le coffre est créé avec ce
    /// mot de passe, qui doit donc être confirmé
    pub new_vault: bool,
    pub error: Option<String>,
    pub focused: bool,
    /// Commande relancée une fois le coffre déverrouillé (ex. enregistrer un utilisateur)
    pub then: Option<Box<Command>>,
}

impl VaultForm {
    pub fn new(mode: VaultMode) -> Self {
        VaultForm {
            mode,
            password: Zeroizing::default(),
            new_password: Zeroizing::default(),
            confirm: Zeroizing::default(),
            reveal: false,
            new_vault: false,
            error: None,
            focused: false,
            then: None,
        }
    }

    /// Déverrouillage, ou création si aucun coffre n'existe encore
    pub fn unlock_or_create(vault_exists: bool) -> Self {
        Self::new(if vault_exists {
            VaultMode::Unlock
        } else {
            VaultMode::Create
        })
    }

    /// Mot de passe maître qui deviendra effectif (création ou changement)
    fn chosen(&self) -> &str {
        match self.mode {
            VaultMode::ChangeMaster => &self.new_password,
            _ => &self.password,
        }
    }

    /// Un nouveau mot de passe maître est choisi : il faut le confirmer
    pub fn creates_password(&self) -> bool {
        match self.mode {
            VaultMode::Create | VaultMode::ChangeMaster => true,
            VaultMode::EnableFileEncryption => self.new_vault,
            _ => false,
        }
    }

    /// Vérifications locales avant d'appeler le cœur (longueur, confirmation)
    pub fn check(&self) -> Result<(), String> {
        if self.password.is_empty() {
            return Err("Saisissez le mot de passe maître".into());
        }
        if self.creates_password() {
            validate_master_password(self.chosen()).map_err(|e| e.to_string())?;
            if self.chosen() != self.confirm.as_str() {
                return Err("La confirmation ne correspond pas au mot de passe".into());
            }
        }
        Ok(())
    }
}

/// Longueur des mots de passe générés
pub const GENERATED_LEN: usize = 20;

/// Caractères des mots de passe générés : sans les caractères ambigus (0 O o 1 l I |)
/// ni ceux qui posent problème dans un shell ou une URL (guillemets, antislash, espace)
pub const PASSWORD_CHARSET: &[u8] =
    b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789!#$%&*+-=?@^_~";

/// Mot de passe aléatoire de [`GENERATED_LEN`] caractères, tiré du générateur du
/// système, avec au moins une majuscule, une minuscule, un chiffre et un symbole
pub fn generate_password() -> Result<Zeroizing<String>, String> {
    // Plus grand multiple de la taille du jeu de caractères : tirage sans biais
    let limit = 256 - 256 % PASSWORD_CHARSET.len();
    loop {
        let mut out = Zeroizing::new(String::with_capacity(GENERATED_LEN));
        let mut buf = Zeroizing::new([0u8; 64]);
        while out.len() < GENERATED_LEN {
            getrandom::fill(buf.as_mut_slice())
                .map_err(|e| format!("Générateur aléatoire indisponible ({e})"))?;
            for &b in buf.iter().filter(|&&b| (b as usize) < limit) {
                if out.len() == GENERATED_LEN {
                    break;
                }
                out.push(PASSWORD_CHARSET[b as usize % PASSWORD_CHARSET.len()] as char);
            }
        }
        let has = |f: fn(&char) -> bool| out.chars().any(|c| f(&c));
        if has(char::is_ascii_uppercase)
            && has(char::is_ascii_lowercase)
            && has(char::is_ascii_digit)
            && has(char::is_ascii_punctuation)
        {
            return Ok(out);
        }
    }
}

/// Découpe une saisie « a, b c » en liste propre
pub fn split_list(value: &str) -> Vec<String> {
    value
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_lists() {
        assert_eq!(
            split_list(" 1.1.1.1, 8.8.8.8  9.9.9.9,,"),
            vec!["1.1.1.1", "8.8.8.8", "9.9.9.9"]
        );
        assert!(split_list("  ").is_empty());
    }

    #[test]
    fn vm_form_roundtrip() {
        let mut f = VmForm::create(None);
        f.name = "web".into();
        f.vlan = "10".into();
        f.port = "2222".into();
        f.tags = "web, docker".into();
        f.dns = "1.1.1.1 9.9.9.9".into();
        let input = f.to_input().unwrap();
        assert_eq!(input.network.vlan, Some(10));
        assert_eq!(input.ssh.port, 2222);
        assert_eq!(input.tags, vec!["web", "docker"]);

        let vm = Vm::from_input(input);
        let back = VmForm::edit(&vm);
        assert_eq!(back.vlan, "10");
        assert_eq!(back.dns, "1.1.1.1, 9.9.9.9");
        assert_eq!(back.edit_id.as_deref(), Some(vm.id.as_str()));
    }

    fn host(id: &str, group: Option<&str>) -> Host {
        let mut h = Host::from_input(vm_core::HostInput {
            name: id.into(),
            group_id: group.map(String::from),
            ..Default::default()
        });
        h.id = id.into();
        h
    }

    #[test]
    fn vm_form_host_imposes_group() {
        let pve = host("pve", Some("lab"));
        let mut f = VmForm::create_on(Some("prod".into()), Some(&pve));
        assert_eq!(f.host_id.as_deref(), Some("pve"));
        assert_eq!(f.group_id.as_deref(), Some("lab"));
        let input = f.to_input().unwrap();
        assert_eq!(input.host_id.as_deref(), Some("pve"));
        assert_eq!(input.group_id.as_deref(), Some("lab"));

        // « Aucune » : la machine est retirée, le groupe reste modifiable
        f.set_host(None);
        assert_eq!(f.host_id, None);
        assert_eq!(f.group_id.as_deref(), Some("lab"));
        f.group_id = Some("prod".into());
        assert_eq!(f.to_input().unwrap().group_id.as_deref(), Some("prod"));

        // Machine sans groupe : la VM devient non classée
        f.set_host(Some(&host("nas", None)));
        assert_eq!(f.group_id, None);
    }

    #[test]
    fn vm_form_sync_host() {
        let mut f = VmForm::create(None);
        f.host_id = Some("pve".into());
        f.group_id = Some("ancien".into());
        f.sync_host(&[host("pve", Some("lab"))]);
        assert_eq!(f.group_id.as_deref(), Some("lab"));
        // Machine supprimée entre-temps : oubliée, le groupe est conservé
        f.sync_host(&[]);
        assert_eq!(f.host_id, None);
        assert_eq!(f.group_id.as_deref(), Some("lab"));
        // Sans machine : rien ne change
        f.group_id = None;
        f.sync_host(&[host("pve", Some("lab"))]);
        assert_eq!(f.group_id, None);
    }

    #[test]
    fn vm_form_numeric_errors_point_to_tab() {
        let mut f = VmForm::create(None);
        f.vlan = "dix".into();
        assert_eq!(f.to_input().unwrap_err().0, VmTab::Network);
        f.vlan.clear();
        f.port = "70000".into();
        assert_eq!(f.to_input().unwrap_err().0, VmTab::Ssh);
        f.port.clear();
        assert_eq!(f.to_input().unwrap().ssh.port, 22);
    }

    #[test]
    fn generated_passwords() {
        let a = generate_password().unwrap();
        let b = generate_password().unwrap();
        assert_ne!(a.as_str(), b.as_str());
        for pw in [&a, &b] {
            assert_eq!(pw.chars().count(), GENERATED_LEN);
            assert!(pw.bytes().all(|c| PASSWORD_CHARSET.contains(&c)));
            assert!(!pw.contains(['0', 'O', 'o', '1', 'l', 'I', '|', '"', '\'', ' ']));
            assert!(pw.chars().any(|c| c.is_ascii_uppercase()));
            assert!(pw.chars().any(|c| c.is_ascii_lowercase()));
            assert!(pw.chars().any(|c| c.is_ascii_digit()));
            assert!(pw.chars().any(|c| c.is_ascii_punctuation()));
        }
        // Jeu de caractères sans doublon
        let mut set = PASSWORD_CHARSET.to_vec();
        set.sort_unstable();
        set.dedup();
        assert_eq!(set.len(), PASSWORD_CHARSET.len());
    }

    #[test]
    fn user_form_password_change() {
        let mut f = UserForm::create("vm");
        assert_eq!(f.password_change(), PasswordChange::Keep);
        f.password = Zeroizing::new("s3cret".into());
        assert_eq!(f.password_change(), PasswordChange::Set("s3cret"));
        // Rien à supprimer sans mot de passe enregistré
        f.remove_password = true;
        assert_eq!(f.password_change(), PasswordChange::Set("s3cret"));
        f.has_password = true;
        assert_eq!(f.password_change(), PasswordChange::Remove);
    }

    #[test]
    fn vault_form_checks() {
        let mut f = VaultForm::new(VaultMode::Create);
        assert!(f.check().is_err());
        f.password = Zeroizing::new("court".into());
        f.confirm = Zeroizing::new("court".into());
        assert!(f.check().unwrap_err().contains('8'));
        f.password = Zeroizing::new("assez-long".into());
        assert!(f.check().unwrap_err().contains("confirmation"));
        f.confirm = Zeroizing::new("assez-long".into());
        assert!(f.check().is_ok());

        let mut f = VaultForm::new(VaultMode::ChangeMaster);
        f.password = Zeroizing::new("x".into());
        f.new_password = Zeroizing::new("nouveau-mdp".into());
        f.confirm = Zeroizing::new("nouveau-mdp".into());
        assert!(f.check().is_ok());
        f.confirm = Zeroizing::new("autre".into());
        assert!(f.check().is_err());

        let mut f = VaultForm::new(VaultMode::Unlock);
        f.password = Zeroizing::new("x".into());
        assert!(f.check().is_ok());

        // Chiffrement du fichier : confirmation exigée seulement si le coffre est à créer
        let mut f = VaultForm::new(VaultMode::EnableFileEncryption);
        f.password = Zeroizing::new("x".into());
        assert!(f.check().is_ok());
        f.new_vault = true;
        assert!(f.check().is_err());
    }

    #[test]
    fn error_tabs() {
        assert_eq!(tab_for_error("Passerelle invalide"), Some(VmTab::Network));
        assert_eq!(tab_for_error("Port SSH invalide : 0"), Some(VmTab::Ssh));
        assert_eq!(
            tab_for_error("Une VM nommée « a » existe déjà"),
            Some(VmTab::General)
        );
    }
}
