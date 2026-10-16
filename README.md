# VM Manager

Mémo / inventaire de machines virtuelles pour homelab et petites infras : groupes de VM, adresses IP et détails réseau, comptes utilisateurs, informations de connexion SSH — le tout stocké dans un simple fichier JSON local.

> **Purement informatif** : l'application ne démarre, n'arrête ni ne contacte aucune VM ni aucun hyperviseur. Elle sert de fiche mémo centralisée.

**100 % Rust** : interface native [egui / eframe](https://github.com/emilk/egui), un seul exécutable, ni Node.js ni navigateur embarqué. Style inspiré de [Server Power Manager](https://github.com/NolanEDUVAUD/server-manager) (barre latérale, cartes, mêmes thèmes de couleurs).

## Fonctionnalités

- **Groupes de VM** : création, modification (nom, description, couleur), liste dépliable des VM membres, suppression — les VM d'un groupe supprimé sont conservées et deviennent « non classées »
- **Fiches VM** : nom, description, groupe, système d'exploitation, étiquettes, notes libres
- **Réseau** : IP (v4 / v6), nom d'hôte, sous-réseau CIDR, passerelle, VLAN, DNS, adresse MAC, pont / interface hyperviseur — avec validation (IP et passerelle dans le sous-réseau, VLAN 1–4094, MAC…)
- **Utilisateurs par VM** : ajout, modification, suppression directement dans la fiche ; nom complet, rôle, droits sudo, notes ; identifiants uniques par VM
- **SSH** : port, utilisateur par défaut, méthode d'authentification (clé, mot de passe, agent…), référence de la clé ou de l'entrée du gestionnaire de mots de passe, notes ; commande `ssh` prête à copier
- **Recherche** multi-mots (nom, IP, VLAN, étiquette, utilisateur…, insensible à la casse et aux accents) et filtre par groupe
- **Tableau de bord** : compteurs, répartition par groupe, VM modifiées récemment
- **Duplication** d'une VM, **export / import JSON** (fusion ou remplacement)
- **Thèmes** changeables à chaud dans Paramètres → Apparence : One Half Dark, Fluent Dark, Gruvbox Dark, Nord, Dracula, Catppuccin Mocha, Tokyo Night (choix mémorisé dans `vm-manager.prefs.json`, à côté de l'inventaire)

## Télécharger un exécutable (sans rien installer)

1. Sur GitHub, onglet **Actions** → workflow **Build** → bouton **Run workflow** (choix des plateformes : Windows, Linux, macOS).
2. Après ~5 à 10 min, onglet **Releases** : une release **brouillon (Draft)** `v0.1.0-build.N` contient les exécutables.
   Les brouillons ne sont visibles que par les mainteneurs du dépôt ; clique *Edit* → *Publish release* pour la rendre publique.
   Les mêmes fichiers sont aussi en bas de la page du run (section **Artifacts**, conservés 14 jours).
   - **Windows** : `vm-manager-windows-<commit>.zip` → décompresse et lance `vm-manager.exe`.
     L'exécutable n'est pas signé : si SmartScreen s'affiche, *Informations complémentaires* → *Exécuter quand même*.
   - **Linux** : `tar xzf vm-manager-linux-<commit>.tar.gz` puis `./vm-manager`.
   - **macOS** : idem ; au premier lancement, clic droit → *Ouvrir* (binaire non signé).

Le fichier `vm-manager.json` est créé à côté de l'exécutable.

## Installation depuis les sources

### Prérequis

- [Rust](https://rustup.rs) stable ≥ 1.95 (`rustup update`).
- **Windows** : Visual C++ Build Tools (MSVC), installés avec rustup.
- **Linux** : rien à compiler en plus. À l'exécution : un serveur X11 ou Wayland avec OpenGL, et `xdg-desktop-portal` pour les fenêtres d'export / import (présent sur les bureaux courants).
- **macOS** : `xcode-select --install`.

### Compiler et lancer

```bash
git clone https://github.com/Neoofix/VM-manager.git
cd VM-manager
cargo run --release
```

L'exécutable est produit dans `target/release/vm-manager` (`vm-manager.exe` sous Windows) : il peut être copié n'importe où, il ne dépend d'aucun autre fichier.

## Où sont stockées les données ?

L'application est **portable** : par défaut, l'inventaire est enregistré dans **`vm-manager.json`, à côté de l'exécutable**. Copier le dossier (exécutable + JSON) sur une clé USB ou un autre PC suffit à tout emporter.

Ordre de priorité pour choisir le fichier :

| Priorité | Source | Exemple |
|---|---|---|
| 1 | Argument `--data` | `vm-manager --data D:\Infra\inventaire.json` |
| 2 | Variable d'environnement `VM_MANAGER_DATA` | `VM_MANAGER_DATA=~/Nextcloud/vms.json vm-manager` |
| 3 | Dossier de l'exécutable | `C:\Outils\VM Manager\vm-manager.json` |
| 4 | Dossier courant, si celui de l'exécutable n'est pas modifiable (ex. `Program Files`, `/usr/bin`) | `./vm-manager.json` |

Le chemin réellement utilisé et sa source sont affichés dans **Paramètres**.

> Avec `cargo run`, l'exécutable est dans `target/…` : le fichier y serait créé (et effacé par `cargo clean`). En développement, préfère `cargo run -- --data ./vm-manager.json`.

Chaque modification est écrite immédiatement, de façon atomique (fichier temporaire puis renommage). Si le fichier est illisible au démarrage, il est **mis de côté** (`vm-manager.json.corrupt-<date>`) plutôt qu'écrasé, et un avertissement s'affiche dans les Paramètres. Un fichier écrit par une version plus récente de l'application est refusé plutôt qu'abîmé.

> ⚠️ **Sécurité** : le fichier JSON n'est **pas chiffré**. N'y enregistre jamais de mot de passe ni de clé privée — note seulement *où* les trouver (chemin de la clé, entrée Bitwarden / KeePass…). C'est l'usage prévu du champ « Référence de la clé / du secret ».

## Utilisation

1. **Groupes** → *Créer un groupe* (ex. « Production », « Lab », « Client A ») et choisis une couleur.
2. **Machines virtuelles** → *Nouvelle VM* : formulaire à trois onglets — *Général*, *Réseau*, *SSH*. Seul le nom est obligatoire. En cas d'erreur, le formulaire bascule sur l'onglet concerné.
3. Clique sur une carte pour ouvrir la **fiche** : toutes les infos, boutons de copie (IP, MAC, commande SSH…), gestion des **utilisateurs**, puis *Modifier*, *Dupliquer* ou *Supprimer*.
4. La recherche accepte plusieurs mots : `prod 192.168.10`, `vlan 20`, `deploy`…
5. **Paramètres** : emplacement du fichier, export / import.

Raccourcis : `Échap` ferme la fenêtre ouverte, `Entrée` valide les formulaires de groupe et d'utilisateur.

## Architecture

```
VM-manager/                     workspace Cargo
├── crates/vm-core/             cœur métier, sans interface graphique
│   └── src/
│       ├── models.rs           Group, Vm, NetworkInfo, SshInfo, VmUser (+ *Input)
│       ├── validation.rs       nettoyage et validation (IP, CIDR, VLAN, MAC, identifiants…)
│       ├── store.rs            persistance JSON transactionnelle, CRUD, import / export
│       ├── query.rs            recherche plein texte, commande SSH
│       └── error.rs            erreurs typées (messages en français)
└── crates/vm-manager/          application de bureau (eframe / egui)
    ├── assets/icon.png
    └── src/
        ├── main.rs             démarrage, fenêtre
        ├── paths.rs            choix du fichier de données (mode portable)
        ├── app.rs              état de l'interface et application des commandes
        ├── forms.rs            états des formulaires (saisie → modèles du cœur)
        ├── theme.rs            7 thèmes, palette active changeable à chaud
        ├── prefs.rs            préférences (thème) à côté du fichier de données
        └── ui/                 écrans : dashboard, vms, vm_detail, vm_form, groups, settings, widgets
```

- Toute la logique métier (validation, unicité des noms, intégrité des références de groupe, écriture atomique) vit dans `vm-core` : elle est testée seule. Une opération qui échoue (validation ou écriture disque) ne modifie ni la mémoire ni le fichier.
- L'interface est en mode immédiat : les écrans lisent l'inventaire et déposent des `Command` (enregistrer, supprimer, dupliquer…) que `app.rs` applique après l'affichage, en renvoyant les erreurs dans le formulaire concerné.

### Règles de gestion

- Noms de groupe et de VM **uniques** (insensible à la casse) ; identifiants d'utilisateur uniques par VM.
- Sous-réseau enregistré sous sa forme canonique (`192.168.10.77/24` → `192.168.10.0/24`) ; l'IP et la passerelle doivent lui appartenir.
- MAC normalisée en minuscules avec `:` ; étiquettes et DNS dédoublonnés.

## Développement

```bash
cargo test --workspace                                  # cœur + logique de l'application
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -- --data ./vm-manager.json                   # lancer avec un inventaire local
```

La CI GitHub Actions (`.github/workflows/ci.yml`) vérifie format, Clippy, tests et compilation release sous Linux et Windows à chaque push sur `main` / `dev`.

## Licence

MIT
