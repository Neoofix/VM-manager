/// États des formulaires : champs texte librement éditables, convertis en
/// saisies `vm-core` à l'enregistrement (la validation métier reste dans le cœur)
use vm_core::{
    Group, GroupInput, NetworkInfo, SshAuthMethod, SshInfo, Vm, VmInput, VmUser, VmUserInput,
    DEFAULT_SSH_PORT,
};

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
    pub error: Option<String>,
    pub focused: bool,
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
            error: None,
            focused: false,
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
    fn error_tabs() {
        assert_eq!(tab_for_error("Passerelle invalide"), Some(VmTab::Network));
        assert_eq!(tab_for_error("Port SSH invalide : 0"), Some(VmTab::Ssh));
        assert_eq!(
            tab_for_error("Une VM nommée « a » existe déjà"),
            Some(VmTab::General)
        );
    }
}
