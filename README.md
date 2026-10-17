# VM Manager

Mémo / inventaire de machines virtuelles pour homelab et petites infras : groupes de VM, adresses IP et détails réseau, comptes utilisateurs, informations de connexion SSH — le tout stocké dans un simple fichier JSON local.

> **Purement informatif** : l'application ne démarre, n'arrête ni ne contacte aucune VM ni aucun hyperviseur. Elle sert de fiche mémo centralisée.

**100 % Rust** : interface native [egui / eframe](https://github.com/emilk/egui), un seul exécutable, ni Node.js ni navigateur embarqué. Style inspiré de [Server Power Manager](https://github.com/NolanEDUVAUD/server-manager) (barre latérale, cartes, mêmes thèmes de couleurs).

## Fonctionnalités

- **Groupes de VM** : création, modification (nom, description, couleur), liste dépliable des VM membres, suppression — les VM d'un groupe supprimé sont conservées et deviennent « non classées »
- **Machines physiques** (hyperviseurs) : nom, groupe, hyperviseur (saisie libre ou liste : Proxmox VE, VMware ESXi, Hyper-V, KVM…), nom d'hôte, IP d'administration, processeur, mémoire vive, stockage, emplacement (salle, baie…), description, notes ; fiche avec la liste des VM hébergées, affectation de VM en quelques cases à cocher
- **Fiches VM** : nom, description, groupe, système d'exploitation, étiquettes, notes libres
- **Réseau** : IP (v4 / v6), nom d'hôte, sous-réseau CIDR, passerelle, VLAN, DNS, adresse MAC, pont / interface hyperviseur — avec validation (IP et passerelle dans le sous-réseau, VLAN 1–4094, MAC…)
- **Utilisateurs par VM** : ajout, modification, suppression directement dans la fiche ; nom complet, rôle, droits sudo, notes ; identifiants uniques par VM
- **Mots de passe chiffrés** : mot de passe de chaque utilisateur enregistré dans un coffre protégé par un mot de passe maître (Argon2id + XChaCha20-Poly1305), générateur de mots de passe, affichage et copie à la demande, verrouillage automatique ; chiffrement facultatif de tout le fichier de données
- **SSH** : port, utilisateur par défaut, méthode d'authentification (clé, mot de passe, agent…), référence de la clé ou de l'entrée du gestionnaire de mots de passe, notes ; commande `ssh` prête à copier
- **Recherche** multi-mots (nom, IP, VLAN, étiquette, utilisateur…, insensible à la casse et aux accents) et filtre par groupe
- **Tableau de bord** : compteurs (VM, machines physiques, groupes, comptes, VLAN), répartition par groupe, VM modifiées récemment
- **Duplication** d'une VM, **export / import JSON** (fusion ou remplacement)
- **Thèmes** changeables à chaud dans Paramètres → Apparence : One Half Dark, Fluent Dark, Gruvbox Dark, Nord, Dracula, Catppuccin Mocha, Tokyo Night (choix mémorisé dans `vm-manager.prefs.json`, à côté de l'inventaire)

## Télécharger et installer (sans compiler)

1. Sur GitHub, onglet **Actions** → workflow **Build** → bouton **Run workflow** (choix des plateformes : Windows, Linux, macOS).
2. Après ~5 à 10 min, onglet **Releases** : une release **brouillon (Draft)** `v0.1.0-build.N` contient les fichiers.
   Les brouillons ne sont visibles que par les mainteneurs ; *Edit* → *Publish release* pour la rendre publique.
   Les mêmes fichiers sont aussi en bas de la page du run (section **Artifacts**, conservés 14 jours).

| Fichier | Plateforme | Installation | Inventaire enregistré dans |
|---|---|---|---|
| `VM-Manager-<version>-x64.msi` | Windows | double-clic → installé dans `Program Files\VM Manager`, raccourci dans le menu Démarrer ; désinstallation via *Paramètres → Applications* | `%APPDATA%\VM Manager\vm-manager.json` |
| `vm-manager_<version>-1_amd64.deb` | Debian / Ubuntu | `sudo apt install ./vm-manager_<version>-1_amd64.deb` → commande `vm-manager` et entrée dans le menu des applications ; `sudo apt remove vm-manager` pour désinstaller | `~/.local/share/vm-manager/vm-manager.json` |
| `vm-manager-windows-<commit>-portable.zip` | Windows | décompresser, lancer `vm-manager.exe` | à côté de l'exécutable |
| `vm-manager-linux-<commit>-portable.tar.gz` | Linux | `tar xzf …` puis `./vm-manager` | à côté de l'exécutable |
| `vm-manager-macos-<commit>-portable.tar.gz` | macOS (Apple Silicon) | idem ; premier lancement : clic droit → *Ouvrir* | à côté de l'exécutable |

Les binaires ne sont pas signés : si Windows SmartScreen s'affiche, *Informations complémentaires* → *Exécuter quand même*.

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

En **version portable** (zip / tar.gz), l'inventaire est enregistré dans **`vm-manager.json`, à côté de l'exécutable**. En **version installée** (`.msi` / `.deb`), il va dans le dossier de données de l'utilisateur. Copier le dossier (exécutable + JSON) sur une clé USB ou un autre PC suffit à tout emporter.

Ordre de priorité pour choisir le fichier :

| Priorité | Source | Exemple |
|---|---|---|
| 1 | Argument `--data` | `vm-manager --data D:\Infra\inventaire.json` |
| 2 | Variable d'environnement `VM_MANAGER_DATA` | `VM_MANAGER_DATA=~/Nextcloud/vms.json vm-manager` |
| 3 | Dossier de l'exécutable (version portable) | `C:\Outils\VM Manager\vm-manager.json` |
| 4 | Dossier de données de l'utilisateur, si l'exécutable est dans un dossier système ou non modifiable (version installée par le `.msi` / `.deb`) | `%APPDATA%\VM Manager\`, `~/.local/share/vm-manager/`, `~/Library/Application Support/VM Manager/` |
| 5 | Dossier courant, en dernier recours | `./vm-manager.json` |

Le chemin réellement utilisé et sa source sont affichés dans **Paramètres**.

> Avec `cargo run`, l'exécutable est dans `target/…` : le fichier y serait créé (et effacé par `cargo clean`). En développement, préfère `cargo run -- --data ./vm-manager.json`.

Chaque modification est écrite immédiatement, de façon atomique (fichier temporaire puis renommage). Si le fichier est illisible au démarrage, il est **mis de côté** (`vm-manager.json.corrupt-<date>`) plutôt qu'écrasé, et un avertissement s'affiche dans les Paramètres. Un fichier écrit par une version plus récente de l'application est refusé plutôt qu'abîmé.

> ⚠️ **Sécurité** : les mots de passe des utilisateurs sont **toujours chiffrés** par le coffre (voir ci-dessous). Le reste de l'inventaire (VM, adresses, notes…) est en clair dans le JSON, sauf si le **chiffrement de tout le fichier** est activé dans *Paramètres → Sécurité*. N'y note jamais de clé privée — seulement *où* la trouver (chemin de la clé, entrée Bitwarden / KeePass…), c'est l'usage prévu du champ « Référence de la clé / du secret ».

## Mots de passe et chiffrement

**Un seul mot de passe maître** protège tout. Il n'est jamais enregistré : Argon2id (64 Mio, 3 passes) en dérive une clé de 256 bits, avec un sel aléatoire conservé dans le fichier, et chaque secret est chiffré par XChaCha20-Poly1305 avec un nonce aléatoire tiré du générateur du système. Une donnée altérée ou un mauvais mot de passe sont détectés (chiffrement authentifié).

- **Créer le coffre** : cadenas 🔒 de la barre de titre, *Paramètres → Sécurité*, ou simplement en enregistrant un premier mot de passe d'utilisateur. Mot de passe maître de 8 caractères minimum, à saisir deux fois.
- **Mots de passe des utilisateurs** : champ *Mot de passe* du formulaire d'utilisateur (masqué, bouton 👁 pour l'afficher, bouton *Générer* : 20 caractères aléatoires sans caractères ambigus). En modification, un champ vide conserve le mot de passe actuel ; *Supprimer le mot de passe* l'efface. Dans la fiche, un utilisateur qui en a un porte la pastille « 🔑 mot de passe » : `••••••••`, 👁 pour l'afficher en clair, bouton de copie.
- **Verrouillage** : le coffre déverrouillé (🔓 dans la barre de titre) garde la clé **uniquement en mémoire**, jamais sur le disque ni dans les journaux. Il se verrouille après **10 minutes sans clavier ni souris**, au clic sur 🔓, et à la fermeture de l'application ; les mots de passe affichés sont alors masqués. Enregistrer un mot de passe coffre verrouillé demande d'abord le mot de passe maître, puis reprend l'enregistrement.
- **Changer le mot de passe maître** (*Paramètres → Sécurité*) : nouveau sel, nouvelle clé, tous les mots de passe (et le fichier s'il est chiffré) sont rechiffrés d'un coup.
- **Chiffrer tout le fichier de données** (*Paramètres → Sécurité*, facultatif) : l'inventaire entier est alors chiffré avec le même mot de passe maître. Il est demandé **à chaque lancement** (écran de déverrouillage), et les exports sont chiffrés eux aussi ; importer un tel export demande le mot de passe maître du fichier. Décocher la case réécrit le fichier en clair (les mots de passe restent chiffrés par le coffre).
- **Import** : les mots de passe d'un fichier dont le coffre est différent ne peuvent pas être déchiffrés ici ; ils sont écartés et leur nombre est indiqué à la fin de l'import.

> ⚠️ **Rien n'est récupérable sans le mot de passe maître** : ni les mots de passe enregistrés, ni l'inventaire s'il est entièrement chiffré. Il n'existe aucune porte dérobée. En cas d'oubli, *Réinitialiser le coffre* supprime le coffre et **tous** les mots de passe enregistrés (le reste de l'inventaire est conservé) ; c'est impossible tant que le fichier est entièrement chiffré, puisqu'il ne peut alors plus être ouvert. Note le mot de passe maître dans un endroit sûr, et garde des exports.

## Organisation : groupe → machine physique → VM

L'inventaire est hiérarchique :

```
Groupe (Production, Lab, Client A…)
└── Machine physique (pve-01, esxi-02…)        facultative
    └── VM (web-01, db-01…)
```

- Une **machine physique** appartient (ou non) à un groupe ; une VM peut être rattachée à une machine physique, ou rester « libre » (directement dans son groupe, ou non classée).
- **Une VM posée sur une machine physique a toujours le groupe de cette machine** : l'affecter à une machine la fait passer dans le groupe de la machine, et changer le groupe d'une machine y emmène toutes ses VM.
- **Retirer** une VM de sa machine la détache sans la déplacer : elle garde son groupe.
- **Supprimer** une machine physique ne supprime aucune VM : elles sont conservées, dans leur groupe, et ne sont plus rattachées à une machine. Supprimer un groupe laisse de même ses machines et ses VM « sans groupe ».

### Page « Machines physiques »

- Les machines sont présentées en cartes, **regroupées par groupe** (puis « Sans groupe ») : nom, hyperviseur, IP ou nom d'hôte, résumé processeur / mémoire / stockage, emplacement et nombre de VM hébergées. Recherche multi-mots (nom, IP, hyperviseur, emplacement, matériel, groupe…) et filtre par groupe, comme pour les VM.
- *Nouvelle machine* ouvre le formulaire : seul le nom est obligatoire ; la mémoire vive se saisit en Go (`64` ou `64 Go`) ; le bouton ⏷ du champ *Hyperviseur* propose les hyperviseurs courants.
- Un clic sur une carte ouvre la **fiche** : toutes les informations (boutons de copie sur l'IP et le nom d'hôte), la liste des VM hébergées (nom, IP, système — un clic ouvre la fiche de la VM), *Retirer* pour détacher une VM, *Affecter des VM* (cases à cocher parmi les VM qui ne sont pas sur cette machine, avec leur groupe et leur machine actuels, et un filtre de recherche), *Modifier* et *Supprimer*.

## Utilisation

1. **Groupes** → *Créer un groupe* (ex. « Production », « Lab », « Client A ») et choisis une couleur.
2. **Machines physiques** → *Nouvelle machine* : enregistre tes hyperviseurs dans leur groupe, puis *Affecter des VM* depuis leur fiche.
3. **Machines virtuelles** → *Nouvelle VM* : formulaire à trois onglets — *Général*, *Réseau*, *SSH*. Seul le nom est obligatoire. En cas d'erreur, le formulaire bascule sur l'onglet concerné.
4. Clique sur une carte pour ouvrir la **fiche** : toutes les infos, boutons de copie (IP, MAC, commande SSH…), gestion des **utilisateurs**, puis *Modifier*, *Dupliquer* ou *Supprimer*.
5. La recherche accepte plusieurs mots : `prod 192.168.10`, `vlan 20`, `deploy`…
6. **Paramètres** : apparence, sécurité (coffre des mots de passe, chiffrement du fichier), emplacement du fichier, export / import.

Raccourcis : `Échap` ferme la fenêtre ouverte, `Entrée` valide les formulaires de groupe et d'utilisateur ainsi que la saisie du mot de passe maître.

## Architecture

```
VM-manager/                     workspace Cargo
├── installer/vm-manager.wxs    installateur Windows (WiX v5)
├── crates/vm-core/             cœur métier, sans interface graphique
│   └── src/
│       ├── models.rs           Group, Host, Vm, NetworkInfo, SshInfo, VmUser (+ *Input)
│       ├── validation.rs       nettoyage et validation (IP, CIDR, VLAN, MAC, identifiants…)
│       ├── store.rs            persistance JSON transactionnelle, CRUD, import / export, coffre
│       ├── vault.rs            mot de passe maître (Argon2id), chiffrement (XChaCha20-Poly1305)
│       ├── query.rs            recherche plein texte, commande SSH
│       └── error.rs            erreurs typées (messages en français)
└── crates/vm-manager/          application de bureau (eframe / egui)
    ├── assets/             icône (png, ico)
    ├── packaging/          entrée de menu Linux (.desktop)
    ├── build.rs            icône intégrée à l'exécutable Windows
    └── src/
        ├── main.rs             démarrage, fenêtre
        ├── paths.rs            choix du fichier de données (mode portable)
        ├── app.rs              état de l'interface et application des commandes
        ├── forms.rs            états des formulaires (saisie → modèles du cœur), générateur de mots de passe
        ├── host_form.rs        formulaire de machine physique, sélection des VM à affecter
        ├── theme.rs            7 thèmes, palette active changeable à chaud
        ├── prefs.rs            préférences (thème) à côté du fichier de données
        └── ui/                 écrans : dashboard, vms, vm_detail, vm_form, hosts, groups, settings, vault, titlebar, widgets
```

- Toute la logique métier (validation, unicité des noms, intégrité des références de groupe, écriture atomique) vit dans `vm-core` : elle est testée seule. Une opération qui échoue (validation ou écriture disque) ne modifie ni la mémoire ni le fichier.
- L'interface est en mode immédiat : les écrans lisent l'inventaire et déposent des `Command` (enregistrer, supprimer, dupliquer…) que `app.rs` applique après l'affichage, en renvoyant les erreurs dans le formulaire concerné.

### Règles de gestion

- Noms de groupe, de machine physique et de VM **uniques** (insensible à la casse) ; identifiants d'utilisateur uniques par VM.
- Sous-réseau enregistré sous sa forme canonique (`192.168.10.77/24` → `192.168.10.0/24`) ; l'IP et la passerelle doivent lui appartenir.
- MAC normalisée en minuscules avec `:` ; étiquettes et DNS dédoublonnés.
- Une VM rattachée à une machine physique a toujours le groupe de la machine (affectation, changement de groupe de la machine) ; supprimer une machine conserve ses VM.

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
