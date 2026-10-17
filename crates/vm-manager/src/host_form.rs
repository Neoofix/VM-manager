/// États des fenêtres des machines physiques : formulaire de création / modification
/// et sélection des VM à affecter (champs texte convertis en `HostInput` à l'enregistrement)
use std::collections::BTreeSet;

use vm_core::{Host, HostInput};

/// Hyperviseurs proposés par le menu du champ « Hyperviseur » (saisie libre possible)
pub const HYPERVISORS: [&str; 8] = [
    "Proxmox VE",
    "VMware ESXi",
    "Microsoft Hyper-V",
    "KVM / libvirt",
    "XCP-ng",
    "Nutanix AHV",
    "VirtualBox",
    "Autre",
];

#[derive(Debug, Clone, Default)]
pub struct HostForm {
    /// `None` = création
    pub edit_id: Option<String>,
    pub error: Option<String>,
    /// Le premier champ a déjà reçu le focus clavier
    pub focused: bool,

    pub name: String,
    pub description: String,
    pub group_id: Option<String>,
    pub hypervisor: String,
    pub hostname: String,
    pub ip: String,
    pub cpu: String,
    /// Mémoire vive en Go, saisie libre (« 64 » ou « 64 Go »)
    pub ram: String,
    pub storage: String,
    pub location: String,
    pub notes: String,
}

impl HostForm {
    pub fn create(group_id: Option<String>) -> Self {
        HostForm {
            group_id,
            ..Default::default()
        }
    }

    pub fn edit(h: &Host) -> Self {
        HostForm {
            edit_id: Some(h.id.clone()),
            error: None,
            focused: false,
            name: h.name.clone(),
            description: h.description.clone(),
            group_id: h.group_id.clone(),
            hypervisor: h.hypervisor.clone(),
            hostname: h.hostname.clone(),
            ip: h.ip.clone(),
            cpu: h.cpu.clone(),
            ram: h.ram_gb.map(|r| r.to_string()).unwrap_or_default(),
            storage: h.storage.clone(),
            location: h.location.clone(),
            notes: h.notes.clone(),
        }
    }

    /// Convertit la mémoire vive ; l'erreur nomme le champ fautif
    pub fn to_input(&self) -> Result<HostInput, String> {
        Ok(HostInput {
            name: self.name.clone(),
            description: self.description.clone(),
            group_id: self.group_id.clone(),
            hypervisor: self.hypervisor.clone(),
            hostname: self.hostname.clone(),
            ip: self.ip.clone(),
            cpu: self.cpu.clone(),
            ram_gb: parse_ram(&self.ram)?,
            storage: self.storage.clone(),
            location: self.location.clone(),
            notes: self.notes.clone(),
        })
    }
}

/// « 64 », « 64 Go » ou « 64 GB » → 64 ; vide → aucune valeur
fn parse_ram(value: &str) -> Result<Option<u32>, String> {
    let v = value.trim();
    if v.is_empty() {
        return Ok(None);
    }
    let lower = v.to_lowercase();
    let digits = lower
        .strip_suffix("go")
        .or_else(|| lower.strip_suffix("gb"))
        .unwrap_or(&lower)
        .trim();
    digits.parse::<u32>().map(Some).map_err(|_| {
        format!("Mémoire vive (RAM) invalide : « {v} » (attendu : un nombre entier de Go, ex. 64)")
    })
}

/// Fenêtre « Affecter des VM » d'une machine physique
#[derive(Debug, Clone, Default)]
pub struct AssignState {
    pub host_id: String,
    /// VM cochées (ordre stable pour la commande)
    pub selected: BTreeSet<String>,
    pub search: String,
    pub error: Option<String>,
    pub focused: bool,
}

impl AssignState {
    pub fn new(host_id: &str) -> Self {
        AssignState {
            host_id: host_id.to_string(),
            ..Default::default()
        }
    }

    pub fn toggle(&mut self, vm_id: &str) {
        if !self.selected.remove(vm_id) {
            self.selected.insert(vm_id.to_string());
        }
    }

    pub fn vm_ids(&self) -> Vec<String> {
        self.selected.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_form_roundtrip() {
        let mut f = HostForm::create(Some("g1".into()));
        f.name = "pve-01".into();
        f.hypervisor = "Proxmox VE".into();
        f.ram = " 128 Go ".into();
        f.ip = "192.168.1.10".into();
        let input = f.to_input().unwrap();
        assert_eq!(input.ram_gb, Some(128));
        assert_eq!(input.group_id.as_deref(), Some("g1"));

        let host = Host::from_input(input);
        let back = HostForm::edit(&host);
        assert_eq!(back.edit_id.as_deref(), Some(host.id.as_str()));
        assert_eq!(back.ram, "128");
        assert_eq!(back.hypervisor, "Proxmox VE");
        assert!(!back.focused);
    }

    #[test]
    fn ram_parsing() {
        assert_eq!(parse_ram(""), Ok(None));
        assert_eq!(parse_ram("  "), Ok(None));
        assert_eq!(parse_ram("64"), Ok(Some(64)));
        assert_eq!(parse_ram("32GB"), Ok(Some(32)));
        assert_eq!(parse_ram("16 go"), Ok(Some(16)));
        for bad in ["beaucoup", "-4", "1.5", "64 To"] {
            let err = parse_ram(bad).unwrap_err();
            assert!(err.contains("Mémoire vive"), "{err}");
            assert!(err.contains(bad), "{err}");
        }
    }

    #[test]
    fn assign_selection() {
        let mut a = AssignState::new("h");
        a.toggle("b");
        a.toggle("a");
        a.toggle("c");
        a.toggle("c");
        assert_eq!(a.vm_ids(), vec!["a", "b"]);
        assert_eq!(a.host_id, "h");
    }
}
