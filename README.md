# VM Manager

Mémo / inventaire de machines virtuelles pour homelab et petites infras : groupes de VM, adresses IP et détails réseau, comptes utilisateurs, informations de connexion SSH — le tout stocké dans un simple fichier JSON local.

> **Purement informatif** : l'application ne démarre, n'arrête ni ne contacte aucune VM ni aucun hyperviseur. Elle sert de fiche mémo centralisée.

Construit avec **Tauri v2** (backend Rust) et **React + TypeScript + Tailwind**, dans le même style que [Server Power Manager](https://github.com/NolanEDUVAUD/server-manager) (interface type Windows 11, thème One Half Dark).

## Fonctionnalités

- **Groupes de VM** : création, modification (nom, description, couleur), liste avec les VM membres, suppression — les VM d'un groupe supprimé sont conservées et deviennent « non classées »
- **Fiches VM** : nom, description, groupe, système d'exploitation, étiquettes, notes libres
- **Réseau** : IP (v4 / v6), nom d'hôte, sous-réseau CIDR, passerelle, VLAN, DNS, adresse MAC, pont / interface hyperviseur — avec validation (IP et passerelle dans le sous-réseau, VLAN 1–4094, MAC…)
- **Utilisateurs par VM** : ajout, modification, suppression ; nom complet, rôle, droits sudo, notes ; identifiants uniques par VM
- **SSH** : port, utilisateur par défaut, méthode d'authentification (clé, mot de passe, agent…), référence de la clé ou de l'entrée du gestionnaire de mots de passe, notes ; commande `ssh` prête à copier
- **Recherche** plein texte (nom, IP, VLAN, étiquette, utilisateur…) et filtre par groupe
- **Tableau de bord** : compteurs, répartition par groupe, VM modifiées récemment
- **Duplication** d'une VM (pratique pour des machines quasi identiques)
- **Export / import JSON** (fusion ou remplacement)

## Installation depuis les sources

### 1. Prérequis

| Outil | Version |
|---|---|
| [Rust](https://rustup.rs) | stable récent (≥ 1.77) |
| [Node.js](https://nodejs.org) | 20 LTS ou plus |
| Dépendances système Tauri | voir ci-dessous |

- **Windows** : Visual C++ Build Tools (MSVC) et WebView2 (déjà présent sur Windows 10/11).
- **Linux (Debian / Ubuntu)** :
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libsoup-3.0-dev build-essential
  ```
- **macOS** : `xcode-select --install`.

Détails : [prérequis Tauri v2](https://v2.tauri.app/start/prerequisites/).

### 2. Récupérer le projet

```bash
git clone https://github.com/Neoofix/VM-manager.git
cd VM-manager
npm install
```

### 3. Lancer en développement

```bash
npm run tauri dev
```

La première compilation Rust prend quelques minutes ; les suivantes sont rapides.

### 4. Construire un installateur

```bash
npm run tauri build
```

Les paquets (`.exe` / `.msi` sous Windows, `.deb` / `.AppImage` / `.rpm` sous Linux, `.dmg` sous macOS) sont générés dans `target/release/bundle/`.

## Utilisation

1. **Groupes** → *Créer un groupe* (ex. « Production », « Lab », « Client A ») et choisis une couleur.
2. **Machines virtuelles** → *Nouvelle VM* : le formulaire a trois onglets — *Général*, *Réseau*, *SSH*. Seul le nom est obligatoire.
3. Clique sur une carte pour ouvrir la **fiche** : toutes les infos, boutons de copie (IP, MAC, commande SSH…), gestion des **utilisateurs** (ajout / modification / suppression en place), puis *Modifier*, *Dupliquer* ou *Supprimer*.
4. La barre de recherche accepte plusieurs mots (`prod 192.168.10`, `vlan 20`, `deploy`…), insensibles à la casse et aux accents.
5. **Paramètres** : emplacement du fichier de données, export / import.

### Où sont stockées les données ?

Dans `data.json`, dans le dossier de données de l'application :

| Système | Emplacement |
|---|---|
| Windows | `%APPDATA%\com.homelab.vm-manager\data.json` |
| Linux | `~/.local/share/com.homelab.vm-manager/data.json` |
| macOS | `~/Library/Application Support/com.homelab.vm-manager/data.json` |

La variable d'environnement `VM_MANAGER_DATA` permet d'utiliser un autre fichier (ex. dans un dossier synchronisé) :

```bash
VM_MANAGER_DATA=~/Nextcloud/vm-inventaire.json npm run tauri dev
```

Chaque modification est écrite immédiatement, de façon atomique (fichier temporaire puis renommage). Si le fichier est illisible au démarrage, il est **mis de côté** (`data.json.corrupt-<date>`) plutôt qu'écrasé, et un avertissement s'affiche dans les Paramètres.

> ⚠️ **Sécurité** : le fichier JSON n'est **pas chiffré**. N'y enregistre jamais de mot de passe ni de clé privée — note seulement *où* les trouver (chemin de la clé, entrée Bitwarden / KeePass…). C'est l'usage prévu du champ « Référence de la clé / du secret ».

## Architecture

```
VM-manager/
├── crates/vm-core/        Cœur Rust, sans dépendance à Tauri
│   └── src/
│       ├── models.rs      Group, Vm, NetworkInfo, SshInfo, VmUser (+ *Input)
│       ├── validation.rs  Nettoyage et validation des saisies (IP, CIDR, VLAN, MAC…)
│       ├── store.rs       Persistance JSON transactionnelle, CRUD, import/export
│       └── error.rs       Erreurs typées (messages en français)
├── src-tauri/             Application Tauri : commandes fines autour de vm-core
│   └── src/commands/      inventory, groups, vms, users
└── src/                   Frontend React + TypeScript + Tailwind
    ├── pages/             Dashboard, Vms, Groups, Settings
    ├── components/        Layout, VmCard, VmDetail, VmForm, VmUsers, GroupForm, Modal…
    ├── stores/            État global (zustand) et notifications
    └── api.ts             Appels aux commandes Tauri
```

Toute la logique métier (validation, unicité des noms, intégrité des références de groupe, écriture atomique) vit dans `vm-core` : elle est testée seule, sans interface graphique. Une opération qui échoue (validation ou écriture disque) ne modifie ni la mémoire ni le fichier.

### Règles de gestion

- Noms de groupe et de VM **uniques** (insensible à la casse) ; identifiants d'utilisateur uniques par VM.
- Sous-réseau enregistré sous sa forme canonique (`192.168.10.77/24` → `192.168.10.0/24`) ; l'IP et la passerelle doivent lui appartenir.
- MAC normalisée en minuscules avec `:` ; étiquettes et DNS dédoublonnés.
- Le format du fichier est versionné (`"version": 1`) : un fichier écrit par une version plus récente est refusé plutôt qu'abîmé.

## Tests

```bash
cargo test -p vm-core        # cœur Rust : validation, persistance, CRUD, import/export
npm test                     # utilitaires frontend (recherche, commande SSH…)
cargo clippy --workspace --all-targets -- -D warnings
npm run build                # vérification TypeScript + build Vite
```

La CI GitHub Actions (`.github/workflows/ci.yml`) exécute ces vérifications à chaque push sur `main` / `dev`.

## Licence

MIT
