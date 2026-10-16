//! Systèmes d'exploitation proposés dans le formulaire des VM, regroupés par famille.
//!
//! Le champ reste libre : ces listes ne servent qu'à le remplir plus vite, et à
//! afficher la famille du système (pastille) sur les cartes et la fiche d'une VM.

/// Famille de systèmes : nom, icône (présente dans les polices d'egui) et systèmes proposés
pub struct OsFamily {
    pub name: &'static str,
    pub icon: &'static str,
    pub systems: &'static [&'static str],
    /// Mots-clés (en minuscules) reconnus dans une saisie libre
    keywords: &'static [&'static str],
}

pub const OS_FAMILIES: [OsFamily; 4] = [
    OsFamily {
        name: "Linux",
        icon: "🐧",
        systems: &[
            "Debian 11",
            "Debian 12",
            "Debian 13",
            "Ubuntu Server 20.04 LTS",
            "Ubuntu Server 22.04 LTS",
            "Ubuntu Server 24.04 LTS",
            "Ubuntu Server 26.04 LTS",
            "Rocky Linux 8",
            "Rocky Linux 9",
            "Rocky Linux 10",
            "AlmaLinux 8",
            "AlmaLinux 9",
            "AlmaLinux 10",
            "RHEL 8",
            "RHEL 9",
            "RHEL 10",
            "CentOS Stream 9",
            "CentOS Stream 10",
            "Fedora Server",
            "openSUSE Leap 15.6",
            "SUSE Linux Enterprise 15",
            "Arch Linux",
            "Alpine Linux 3.x",
            "Kali Linux",
            "Proxmox VE 8",
            "Proxmox VE 9",
            "TrueNAS SCALE",
        ],
        keywords: &[
            "linux", "debian", "ubuntu", "rocky", "alma", "rhel", "red hat", "centos", "fedora",
            "suse", "arch", "alpine", "kali", "proxmox", "truenas", "mint", "gentoo", "nixos",
        ],
    },
    OsFamily {
        name: "Windows",
        icon: "⊞",
        systems: &[
            "Windows Server 2016",
            "Windows Server 2019",
            "Windows Server 2022",
            "Windows Server 2025",
            "Windows 10",
            "Windows 11",
        ],
        keywords: &["windows"],
    },
    OsFamily {
        name: "BSD",
        icon: "😈",
        systems: &["FreeBSD 13", "FreeBSD 14", "OpenBSD", "pfSense", "OPNsense"],
        keywords: &["bsd", "pfsense", "opnsense"],
    },
    OsFamily {
        name: "Autres",
        icon: "💿",
        systems: &["VMware ESXi 8", "macOS", "Autre"],
        keywords: &["esxi", "vmware", "macos", "mac os"],
    },
];

/// Famille d'un système saisi librement (`None` si inconnue, vide ou « Autre »)
pub fn family_of(os: &str) -> Option<&'static OsFamily> {
    let os = os.trim().to_lowercase();
    if os.is_empty() || os == "autre" {
        return None;
    }
    OS_FAMILIES
        .iter()
        .find(|f| f.systems.iter().any(|s| s.to_lowercase() == os))
        .or_else(|| {
            OS_FAMILIES
                .iter()
                .find(|f| f.keywords.iter().any(|k| os.contains(k)))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_duplicate_entries() {
        let mut seen = HashSet::new();
        for f in &OS_FAMILIES {
            assert!(!f.systems.is_empty(), "{}", f.name);
            for s in f.systems {
                assert!(seen.insert(s.to_lowercase()), "doublon : {s}");
            }
        }
        let names: HashSet<_> = OS_FAMILIES.iter().map(|f| f.name).collect();
        assert_eq!(names.len(), OS_FAMILIES.len());
    }

    #[test]
    fn every_preset_belongs_to_its_own_family() {
        for f in &OS_FAMILIES {
            for s in f.systems.iter().filter(|s| **s != "Autre") {
                assert_eq!(family_of(s).map(|x| x.name), Some(f.name), "{s}");
            }
        }
    }

    #[test]
    fn free_text_is_recognized() {
        assert_eq!(
            family_of("debian 12 (bookworm)").map(|f| f.name),
            Some("Linux")
        );
        assert_eq!(family_of("Win Server 2022").map(|f| f.name), None);
        assert_eq!(
            family_of("windows server 2022 core").map(|f| f.name),
            Some("Windows")
        );
        assert_eq!(family_of("NetBSD 10").map(|f| f.name), Some("BSD"));
        assert!(family_of("").is_none());
        assert!(family_of("Autre").is_none());
    }
}
