/// Fonctions de lecture de l'inventaire : recherche et commande SSH
use crate::models::{Group, Vm};

/// Adresse à utiliser pour se connecter : IP, sinon nom d'hôte
pub fn vm_host(vm: &Vm) -> &str {
    if vm.network.ip.is_empty() {
        &vm.network.hostname
    } else {
        &vm.network.ip
    }
}

/// Commande SSH prête à copier, ou `None` si la VM n'a ni IP ni nom d'hôte.
/// `user` remplace l'utilisateur par défaut de la VM.
pub fn ssh_command(vm: &Vm, user: Option<&str>) -> Option<String> {
    let host = vm_host(vm);
    if host.is_empty() {
        return None;
    }
    let login = user.unwrap_or(&vm.ssh.default_user);
    let target = if login.is_empty() {
        host.to_string()
    } else {
        format!("{login}@{host}")
    };
    Some(if vm.ssh.port != 22 {
        format!("ssh -p {} {target}", vm.ssh.port)
    } else {
        format!("ssh {target}")
    })
}

/// Recherche plein texte : chaque mot de la requête doit apparaître dans au moins
/// un champ utile (insensible à la casse et aux accents courants)
pub fn vm_matches(vm: &Vm, group: Option<&Group>, query: &str) -> bool {
    let query = fold(query);
    let terms: Vec<&str> = query.split_whitespace().collect();
    if terms.is_empty() {
        return true;
    }
    let vlan = vm
        .network
        .vlan
        .map(|v| format!("vlan {v}"))
        .unwrap_or_default();
    let mut fields: Vec<&str> = vec![
        &vm.name,
        &vm.description,
        &vm.os,
        &vm.notes,
        &vm.network.ip,
        &vm.network.hostname,
        &vm.network.subnet,
        &vm.network.gateway,
        &vm.network.mac,
        &vlan,
        &vm.ssh.default_user,
    ];
    if let Some(g) = group {
        fields.push(&g.name);
    }
    fields.extend(vm.tags.iter().map(String::as_str));
    fields.extend(vm.users.iter().map(|u| u.username.as_str()));
    let haystack: Vec<String> = fields
        .iter()
        .filter(|f| !f.is_empty())
        .map(|f| fold(f))
        .collect();
    terms.iter().all(|t| haystack.iter().any(|h| h.contains(t)))
}

/// Minuscules et lettres accentuées françaises ramenées à leur forme de base
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GroupInput, VmInput, VmUser, VmUserInput};

    fn vm() -> Vm {
        Vm::from_input(VmInput {
            name: "web-01".into(),
            ..Default::default()
        })
    }

    #[test]
    fn ssh_command_variants() {
        let mut v = vm();
        assert_eq!(ssh_command(&v, None), None);
        v.network.hostname = "web.lan".into();
        assert_eq!(ssh_command(&v, None).unwrap(), "ssh web.lan");
        v.network.ip = "10.0.0.5".into();
        v.ssh.default_user = "deploy".into();
        assert_eq!(ssh_command(&v, None).unwrap(), "ssh deploy@10.0.0.5");
        v.ssh.port = 2222;
        assert_eq!(
            ssh_command(&v, Some("root")).unwrap(),
            "ssh -p 2222 root@10.0.0.5"
        );
    }

    #[test]
    fn search_matches_all_terms_across_fields() {
        let mut v = vm();
        v.description = "Serveur d'intégration".into();
        v.tags = vec!["Production".into()];
        v.network.ip = "192.168.10.4".into();
        v.network.vlan = Some(30);
        v.users.push(VmUser::from_input(VmUserInput {
            username: "deploy".into(),
            ..Default::default()
        }));
        let g = Group::from_input(GroupInput {
            name: "Clients".into(),
            ..Default::default()
        });

        assert!(vm_matches(&v, None, ""));
        assert!(vm_matches(&v, None, "production 192.168"));
        assert!(vm_matches(&v, None, "INTEGRATION"));
        assert!(vm_matches(&v, None, "vlan 30"));
        assert!(vm_matches(&v, None, "deploy"));
        assert!(vm_matches(&v, Some(&g), "clients"));
        assert!(!vm_matches(&v, None, "clients"));
        assert!(!vm_matches(&v, None, "production nope"));
    }
}
