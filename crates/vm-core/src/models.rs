/// Modèles de données de l'inventaire (sérialisés en JSON, partagés avec le frontend)
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::vault::{EncryptedBlob, VaultHeader};

/// Version du format du fichier `data.json`, incrémentée à chaque changement incompatible
pub const SCHEMA_VERSION: u32 = 1;

pub const DEFAULT_SSH_PORT: u16 = 22;

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn default_ssh_port() -> u16 {
    DEFAULT_SSH_PORT
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

// ── Racine du fichier de données ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppData {
    #[serde(default = "default_schema_version")]
    pub version: u32,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub vms: Vec<Vm>,
    /// Coffre des mots de passe (absent tant qu'aucun mot de passe maître n'a été défini)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultHeader>,
}

impl Default for AppData {
    fn default() -> Self {
        AppData {
            version: SCHEMA_VERSION,
            groups: Vec::new(),
            vms: Vec::new(),
            vault: None,
        }
    }
}

// ── Groupes ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Couleur d'accent au format `#rrggbb`
    #[serde(default)]
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Saisie d'un groupe (création et modification)
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GroupInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub color: Option<String>,
}

impl Group {
    pub fn from_input(input: GroupInput) -> Self {
        let now = Utc::now();
        Group {
            id: new_id(),
            name: input.name,
            description: input.description,
            color: input.color,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn apply(&mut self, input: GroupInput) {
        self.name = input.name;
        self.description = input.description;
        self.color = input.color;
        self.updated_at = Utc::now();
    }
}

// ── Réseau ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NetworkInfo {
    /// Adresse IPv4 ou IPv6 principale
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub hostname: String,
    /// Identifiant de VLAN 802.1Q (1 à 4094)
    #[serde(default)]
    pub vlan: Option<u16>,
    /// Sous-réseau en notation CIDR (`192.168.10.0/24`)
    #[serde(default)]
    pub subnet: String,
    #[serde(default)]
    pub gateway: String,
    #[serde(default)]
    pub dns: Vec<String>,
    #[serde(default)]
    pub mac: String,
    /// Pont / interface côté hyperviseur (vmbr0, vSwitch…)
    #[serde(default)]
    pub bridge: String,
}

// ── SSH ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SshAuthMethod {
    #[default]
    Key,
    Password,
    Agent,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SshInfo {
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    #[serde(default)]
    pub default_user: String,
    #[serde(default)]
    pub auth_method: SshAuthMethod,
    /// Emplacement de la clé ou nom de l'entrée du gestionnaire de mots de passe —
    /// jamais le secret lui-même
    #[serde(default)]
    pub key_reference: String,
    #[serde(default)]
    pub notes: String,
}

impl Default for SshInfo {
    fn default() -> Self {
        SshInfo {
            port: DEFAULT_SSH_PORT,
            default_user: String::new(),
            auth_method: SshAuthMethod::default(),
            key_reference: String::new(),
            notes: String::new(),
        }
    }
}

// ── Utilisateurs d'une VM ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VmUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub full_name: String,
    /// Rôle libre (admin, service, déploiement…)
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub sudo: bool,
    #[serde(default)]
    pub notes: String,
    /// Mot de passe du compte, chiffré par le coffre — jamais en clair
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<EncryptedBlob>,
}

/// Saisie d'un utilisateur ; le mot de passe se gère à part, via le coffre
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VmUserInput {
    pub username: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub sudo: bool,
    #[serde(default)]
    pub notes: String,
}

impl VmUser {
    pub fn from_input(input: VmUserInput) -> Self {
        VmUser {
            id: new_id(),
            username: input.username,
            full_name: input.full_name,
            role: input.role,
            sudo: input.sudo,
            notes: input.notes,
            password: None,
        }
    }

    /// Met à jour les champs saisis ; le mot de passe chiffré est conservé
    pub fn apply(&mut self, input: VmUserInput) {
        self.username = input.username;
        self.full_name = input.full_name;
        self.role = input.role;
        self.sudo = input.sudo;
        self.notes = input.notes;
    }
}

// ── Machines virtuelles ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vm {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Groupe d'appartenance (une VM sans groupe est « non classée »)
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub network: NetworkInfo,
    #[serde(default)]
    pub ssh: SshInfo,
    #[serde(default)]
    pub users: Vec<VmUser>,
    #[serde(default)]
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Saisie d'une VM (création et modification). Les utilisateurs se gèrent à part
/// pour qu'une modification du formulaire principal n'écrase pas la liste.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VmInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub network: NetworkInfo,
    #[serde(default)]
    pub ssh: SshInfo,
    #[serde(default)]
    pub notes: String,
}

impl Vm {
    pub fn from_input(input: VmInput) -> Self {
        let now = Utc::now();
        Vm {
            id: new_id(),
            name: input.name,
            description: input.description,
            group_id: input.group_id,
            os: input.os,
            tags: input.tags,
            network: input.network,
            ssh: input.ssh,
            users: Vec::new(),
            notes: input.notes,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn apply(&mut self, input: VmInput) {
        self.name = input.name;
        self.description = input.description;
        self.group_id = input.group_id;
        self.os = input.os;
        self.tags = input.tags;
        self.network = input.network;
        self.ssh = input.ssh;
        self.notes = input.notes;
        self.touch();
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}
