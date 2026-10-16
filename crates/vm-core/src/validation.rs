/// Validation et normalisation des saisies.
///
/// Chaque fonction `normalize_*` nettoie la saisie (espaces, doublons, champs vides)
/// puis la valide ; elle renvoie la version propre à enregistrer.
use std::net::IpAddr;

use crate::error::{CoreError, CoreResult};
use crate::models::{GroupInput, VmInput, VmUserInput};

pub const MAX_NAME_LEN: usize = 64;
pub const MAX_USERNAME_LEN: usize = 64;

// ── Groupes ───────────────────────────────────────────────────────────────

pub fn normalize_group(mut input: GroupInput) -> CoreResult<GroupInput> {
    input.name = input.name.trim().to_string();
    input.description = input.description.trim().to_string();
    input.color = non_empty(input.color.as_deref());

    validate_name(&input.name, "Le nom du groupe")?;
    if let Some(color) = &input.color {
        if !is_hex_color(color) {
            return Err(CoreError::validation(format!(
                "Couleur invalide : « {color} » (format attendu : #rrggbb)"
            )));
        }
        input.color = Some(color.to_ascii_lowercase());
    }
    Ok(input)
}

// ── VM ────────────────────────────────────────────────────────────────────

pub fn normalize_vm(mut input: VmInput) -> CoreResult<VmInput> {
    input.name = input.name.trim().to_string();
    input.description = input.description.trim().to_string();
    input.os = input.os.trim().to_string();
    input.notes = input.notes.trim_end().to_string();
    input.group_id = non_empty(input.group_id.as_deref());
    input.tags = dedupe(input.tags.iter().map(|t| t.trim().to_string()));

    validate_name(&input.name, "Le nom de la VM")?;

    // Réseau
    let net = &mut input.network;
    net.ip = net.ip.trim().to_string();
    net.hostname = net.hostname.trim().to_string();
    net.subnet = net.subnet.trim().to_string();
    net.gateway = net.gateway.trim().to_string();
    net.mac = net.mac.trim().to_ascii_lowercase().replace('-', ":");
    net.bridge = net.bridge.trim().to_string();
    net.dns = dedupe(net.dns.iter().map(|d| d.trim().to_string()));

    let ip = parse_ip_field(&net.ip, "Adresse IP")?;
    let gateway = parse_ip_field(&net.gateway, "Passerelle")?;
    for dns in &net.dns {
        parse_ip_field(dns, "Serveur DNS")?;
    }
    if !net.hostname.is_empty() && !is_valid_hostname(&net.hostname) {
        return Err(CoreError::validation(format!(
            "Nom d'hôte invalide : « {} »",
            net.hostname
        )));
    }
    if let Some(vlan) = net.vlan {
        if !(1..=4094).contains(&vlan) {
            return Err(CoreError::validation(format!(
                "VLAN invalide : {vlan} (attendu entre 1 et 4094)"
            )));
        }
    }
    if !net.mac.is_empty() && !is_valid_mac(&net.mac) {
        return Err(CoreError::validation(format!(
            "Adresse MAC invalide : « {} » (format attendu : aa:bb:cc:dd:ee:ff)",
            net.mac
        )));
    }
    if !net.subnet.is_empty() {
        let cidr = Cidr::parse(&net.subnet).ok_or_else(|| {
            CoreError::validation(format!(
                "Sous-réseau invalide : « {} » (format attendu : 192.168.1.0/24)",
                net.subnet
            ))
        })?;
        // Enregistré sous sa forme canonique (bits d'hôte à zéro)
        net.subnet = cidr.to_string();
        for (addr, label) in [(ip, "L'adresse IP"), (gateway, "La passerelle")] {
            if let Some(addr) = addr {
                if !cidr.contains(addr) {
                    return Err(CoreError::validation(format!(
                        "{label} {addr} n'appartient pas au sous-réseau {cidr}"
                    )));
                }
            }
        }
    }

    // SSH
    let ssh = &mut input.ssh;
    ssh.default_user = ssh.default_user.trim().to_string();
    ssh.key_reference = ssh.key_reference.trim().to_string();
    ssh.notes = ssh.notes.trim_end().to_string();
    if ssh.port == 0 {
        return Err(CoreError::validation(
            "Port SSH invalide : 0 (attendu entre 1 et 65535)",
        ));
    }
    if !ssh.default_user.is_empty() {
        validate_username(&ssh.default_user)?;
    }

    Ok(input)
}

// ── Utilisateurs ──────────────────────────────────────────────────────────

pub fn normalize_user(mut input: VmUserInput) -> CoreResult<VmUserInput> {
    input.username = input.username.trim().to_string();
    input.full_name = input.full_name.trim().to_string();
    input.role = input.role.trim().to_string();
    input.notes = input.notes.trim_end().to_string();
    validate_username(&input.username)?;
    Ok(input)
}

pub fn validate_username(username: &str) -> CoreResult<()> {
    if username.is_empty() {
        return Err(CoreError::validation("Le nom d'utilisateur est requis"));
    }
    if username.chars().count() > MAX_USERNAME_LEN {
        return Err(CoreError::validation(format!(
            "Nom d'utilisateur trop long ({MAX_USERNAME_LEN} caractères maximum)"
        )));
    }
    // Couvre les comptes Linux (`deploy`, `www-data`, `svc$`) et Windows (`DOMAINE\admin`, `admin@corp`)
    let ok_chars = username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_.-@\\$".contains(c));
    if !ok_chars || username.starts_with('-') {
        return Err(CoreError::validation(format!(
            "Nom d'utilisateur invalide : « {username} » (lettres, chiffres et _ . - @ \\ $ uniquement)"
        )));
    }
    Ok(())
}

// ── Aides ─────────────────────────────────────────────────────────────────

fn validate_name(name: &str, label: &str) -> CoreResult<()> {
    if name.is_empty() {
        return Err(CoreError::validation(format!("{label} est requis")));
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(CoreError::validation(format!(
            "{label} est trop long ({MAX_NAME_LEN} caractères maximum)"
        )));
    }
    Ok(())
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(String::from)
}

/// Retire les valeurs vides et les doublons (insensible à la casse) en gardant l'ordre
fn dedupe(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for v in values.filter(|v| !v.is_empty()) {
        if !out.iter().any(|o| o.eq_ignore_ascii_case(&v)) {
            out.push(v);
        }
    }
    out
}

fn parse_ip_field(value: &str, label: &str) -> CoreResult<Option<IpAddr>> {
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<IpAddr>()
        .map(Some)
        .map_err(|_| CoreError::validation(format!("{label} invalide : « {value} »")))
}

pub fn is_hex_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

pub fn is_valid_mac(value: &str) -> bool {
    let parts: Vec<&str> = value.split(':').collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

pub fn is_valid_hostname(value: &str) -> bool {
    let value = value.strip_suffix('.').unwrap_or(value);
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

/// Sous-réseau IPv4 ou IPv6 en notation CIDR
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix: u8,
}

impl Cidr {
    pub fn parse(value: &str) -> Option<Cidr> {
        let (addr, prefix) = value.split_once('/')?;
        let addr: IpAddr = addr.trim().parse().ok()?;
        let prefix: u8 = prefix.trim().parse().ok()?;
        let max = if addr.is_ipv4() { 32 } else { 128 };
        if prefix > max {
            return None;
        }
        Some(Cidr {
            network: mask(addr, prefix),
            prefix,
        })
    }

    pub fn contains(&self, addr: IpAddr) -> bool {
        addr.is_ipv4() == self.network.is_ipv4() && mask(addr, self.prefix) == self.network
    }
}

impl std::fmt::Display for Cidr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix)
    }
}

fn mask(addr: IpAddr, prefix: u8) -> IpAddr {
    match addr {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4);
            let m = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            IpAddr::V4((bits & m).into())
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6);
            let m = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            IpAddr::V6((bits & m).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NetworkInfo, SshInfo};

    fn vm(name: &str) -> VmInput {
        VmInput {
            name: name.into(),
            ..Default::default()
        }
    }

    #[test]
    fn group_name_is_trimmed_and_required() {
        let g = normalize_group(GroupInput {
            name: "  Prod  ".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(g.name, "Prod");
        assert!(normalize_group(GroupInput::default()).is_err());
    }

    #[test]
    fn group_color_must_be_hex() {
        let ok = normalize_group(GroupInput {
            name: "a".into(),
            color: Some("#61AFEF".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(ok.color.as_deref(), Some("#61afef"));
        let blank = normalize_group(GroupInput {
            name: "a".into(),
            color: Some("  ".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(blank.color, None);
        assert!(normalize_group(GroupInput {
            name: "a".into(),
            color: Some("red".into()),
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn vm_network_is_validated_and_canonicalised() {
        let mut input = vm("web-01");
        input.network = NetworkInfo {
            ip: " 192.168.10.12 ".into(),
            subnet: "192.168.10.77/24".into(),
            gateway: "192.168.10.1".into(),
            vlan: Some(10),
            mac: "AA-BB-CC-DD-EE-FF".into(),
            dns: vec!["1.1.1.1".into(), "1.1.1.1".into(), "".into()],
            ..Default::default()
        };
        let out = normalize_vm(input).unwrap();
        assert_eq!(out.network.ip, "192.168.10.12");
        assert_eq!(out.network.subnet, "192.168.10.0/24");
        assert_eq!(out.network.mac, "aa:bb:cc:dd:ee:ff");
        assert_eq!(out.network.dns, vec!["1.1.1.1"]);
    }

    #[test]
    fn vm_rejects_bad_network_values() {
        type Mutation = Box<dyn Fn(&mut NetworkInfo)>;
        let cases: Vec<Mutation> = vec![
            Box::new(|n| n.ip = "300.1.1.1".into()),
            Box::new(|n| n.gateway = "gw".into()),
            Box::new(|n| n.vlan = Some(0)),
            Box::new(|n| n.vlan = Some(4095)),
            Box::new(|n| n.subnet = "10.0.0.0/33".into()),
            Box::new(|n| n.subnet = "10.0.0.0".into()),
            Box::new(|n| n.mac = "aa:bb:cc".into()),
            Box::new(|n| n.hostname = "bad_host".into()),
            Box::new(|n| n.dns = vec!["dns.example".into()]),
            Box::new(|n| {
                n.subnet = "10.0.0.0/24".into();
                n.ip = "10.0.1.5".into();
            }),
            Box::new(|n| {
                n.subnet = "10.0.0.0/24".into();
                n.gateway = "192.168.0.1".into();
            }),
        ];
        for (i, mutate) in cases.iter().enumerate() {
            let mut input = vm("x");
            mutate(&mut input.network);
            assert!(
                normalize_vm(input).is_err(),
                "le cas {i} devrait être refusé"
            );
        }
    }

    #[test]
    fn vm_accepts_ipv6() {
        let mut input = vm("v6");
        input.network.ip = "2001:db8::10".into();
        input.network.subnet = "2001:db8::/64".into();
        input.network.gateway = "2001:db8::1".into();
        assert!(normalize_vm(input).is_ok());
    }

    #[test]
    fn vm_ssh_is_validated() {
        let mut input = vm("x");
        input.ssh = SshInfo {
            port: 0,
            ..Default::default()
        };
        assert!(normalize_vm(input.clone()).is_err());
        input.ssh.port = 2222;
        input.ssh.default_user = "bad user".into();
        assert!(normalize_vm(input.clone()).is_err());
        input.ssh.default_user = "deploy".into();
        assert!(normalize_vm(input).is_ok());
    }

    #[test]
    fn vm_tags_are_deduplicated() {
        let mut input = vm("x");
        input.tags = vec!["web".into(), " Web ".into(), "".into(), "db".into()];
        assert_eq!(normalize_vm(input).unwrap().tags, vec!["web", "db"]);
    }

    #[test]
    fn usernames() {
        for ok in [
            "root",
            "www-data",
            "svc$",
            "CORP\\admin",
            "admin@corp",
            "_apt",
        ] {
            assert!(validate_username(ok).is_ok(), "{ok}");
        }
        for bad in ["", "a b", "-x", "é", "a/b"] {
            assert!(validate_username(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn cidr_contains() {
        let c = Cidr::parse("10.1.0.0/16").unwrap();
        assert!(c.contains("10.1.200.3".parse().unwrap()));
        assert!(!c.contains("10.2.0.1".parse().unwrap()));
        assert!(!c.contains("::1".parse().unwrap()));
        assert_eq!(Cidr::parse("0.0.0.0/0").unwrap().to_string(), "0.0.0.0/0");
    }
}
