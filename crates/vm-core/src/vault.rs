/// Coffre-fort des mots de passe et chiffrement du fichier de données.
///
/// Un seul mot de passe maître sert à tout : Argon2id en dérive une clé de 32
/// octets (sel aléatoire conservé dans l'en-tête du coffre), et XChaCha20-Poly1305
/// chiffre chaque secret avec un nonce aléatoire de 24 octets tiré du générateur
/// du système. Les mots de passe ne sont jamais écrits en clair, et la clé comme
/// les textes déchiffrés sont effacés de la mémoire dès qu'ils ne servent plus.
use std::fmt;

use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::{CoreError, CoreResult};

/// Version du format de l'en-tête du coffre
pub const VAULT_VERSION: u32 = 1;

/// Identifiant du format d'un fichier de données chiffré
pub const ENCRYPTED_FILE_FORMAT: &str = "vm-manager-encrypted";

/// Version du format de l'enveloppe chiffrée
pub const ENCRYPTED_FILE_VERSION: u32 = 1;

/// Longueur minimale du mot de passe maître (en caractères)
pub const MIN_MASTER_LEN: usize = 8;

const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;

/// Texte connu chiffré dans l'en-tête : le déchiffrer prouve que la clé est la bonne
const CHECK_PLAINTEXT: &str = "vm-manager-vault-check";

// ── Paramètres de dérivation ──────────────────────────────────────────────

/// Coût d'Argon2id, conservé avec le sel pour pouvoir le faire évoluer
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct KdfParams {
    /// Mémoire utilisée, en Kio
    pub m_cost_kib: u32,
    /// Nombre de passes
    pub t_cost: u32,
    /// Degré de parallélisme
    pub p_cost: u32,
}

impl Default for KdfParams {
    /// Réglage de production : 64 Mio, 3 passes, 1 fil
    fn default() -> Self {
        KdfParams {
            m_cost_kib: 64 * 1024,
            t_cost: 3,
            p_cost: 1,
        }
    }
}

impl KdfParams {
    /// Réglage minimal, très rapide — réservé aux tests, jamais pour de vraies données
    pub fn insecure_for_tests() -> Self {
        KdfParams {
            m_cost_kib: 64,
            t_cost: 1,
            p_cost: 1,
        }
    }
}

// ── Clé dérivée ───────────────────────────────────────────────────────────

/// Clé de chiffrement dérivée du mot de passe maître, effacée de la mémoire à sa libération
#[derive(Clone)]
pub struct VaultKey(Zeroizing<[u8; KEY_LEN]>);

impl fmt::Debug for VaultKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("VaultKey(***)")
    }
}

impl VaultKey {
    fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new_from_slice(self.0.as_slice()).expect("clé de 32 octets")
    }
}

/// Vérifie la robustesse minimale du mot de passe maître
pub fn validate_master_password(master: &str) -> CoreResult<()> {
    if master.chars().count() < MIN_MASTER_LEN {
        return Err(CoreError::validation(format!(
            "Le mot de passe maître doit contenir au moins {MIN_MASTER_LEN} caractères"
        )));
    }
    Ok(())
}

fn derive_key(master: &str, salt: &[u8], params: KdfParams) -> CoreResult<VaultKey> {
    let argon_params = Params::new(
        params.m_cost_kib,
        params.t_cost,
        params.p_cost,
        Some(KEY_LEN),
    )
    .map_err(|e| CoreError::Crypto(format!("paramètres Argon2 invalides ({e})")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(master.as_bytes(), salt, key.as_mut_slice())
        .map_err(|e| CoreError::Crypto(format!("dérivation de clé impossible ({e})")))?;
    Ok(VaultKey(key))
}

fn random_bytes<const N: usize>() -> CoreResult<[u8; N]> {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf)
        .map_err(|e| CoreError::Crypto(format!("générateur aléatoire indisponible ({e})")))?;
    Ok(buf)
}

fn decode_b64(value: &str, what: &str) -> CoreResult<Vec<u8>> {
    B64.decode(value)
        .map_err(|_| CoreError::Crypto(format!("{what} mal encodé")))
}

// ── Chiffrement d'un secret ───────────────────────────────────────────────

/// Secret chiffré : nonce et texte chiffré (étiquette d'authentification comprise), en base64
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedBlob {
    pub nonce: String,
    pub data: String,
}

fn encrypt_bytes(key: &VaultKey, plaintext: &[u8]) -> CoreResult<([u8; NONCE_LEN], Vec<u8>)> {
    let nonce = random_bytes::<NONCE_LEN>()?;
    let data = key
        .cipher()
        .encrypt(&XNonce::from(nonce), plaintext)
        .map_err(|_| CoreError::Crypto("chiffrement impossible".into()))?;
    Ok((nonce, data))
}

/// Déchiffre ; une clé fausse ou une donnée altérée donne `None`
fn decrypt_bytes(
    key: &VaultKey,
    nonce: &str,
    data: &str,
) -> CoreResult<Option<Zeroizing<Vec<u8>>>> {
    let nonce = decode_b64(nonce, "Nonce")?;
    let nonce: [u8; NONCE_LEN] = nonce
        .try_into()
        .map_err(|_| CoreError::Crypto("nonce de longueur invalide".into()))?;
    let data = decode_b64(data, "Texte chiffré")?;
    Ok(key
        .cipher()
        .decrypt(&XNonce::from(nonce), data.as_slice())
        .ok()
        .map(Zeroizing::new))
}

/// Chiffre un secret avec un nonce aléatoire neuf
pub fn encrypt(key: &VaultKey, plaintext: &str) -> CoreResult<EncryptedBlob> {
    let (nonce, data) = encrypt_bytes(key, plaintext.as_bytes())?;
    Ok(EncryptedBlob {
        nonce: B64.encode(nonce),
        data: B64.encode(data),
    })
}

/// Déchiffre un secret ; échoue si la clé n'est pas la bonne ou si la donnée a été altérée
pub fn decrypt(key: &VaultKey, blob: &EncryptedBlob) -> CoreResult<Zeroizing<String>> {
    let bytes = decrypt_bytes(key, &blob.nonce, &blob.data)?.ok_or_else(|| {
        CoreError::Crypto("déchiffrement impossible (clé incorrecte ou donnée altérée)".into())
    })?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| CoreError::Crypto("texte déchiffré invalide".into()))?;
    Ok(Zeroizing::new(text.to_owned()))
}

// ── En-tête du coffre ─────────────────────────────────────────────────────

/// En-tête du coffre, enregistré dans le fichier de données
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultHeader {
    pub version: u32,
    /// Sel d'Argon2id, en base64
    pub salt: String,
    pub kdf: KdfParams,
    /// Texte connu chiffré, pour reconnaître un mauvais mot de passe maître
    pub check: EncryptedBlob,
}

impl VaultHeader {
    /// Crée un coffre neuf (sel aléatoire) et renvoie la clé dérivée de `master`
    pub fn create(master: &str, params: KdfParams) -> CoreResult<(VaultHeader, VaultKey)> {
        validate_master_password(master)?;
        let salt = random_bytes::<SALT_LEN>()?;
        let key = derive_key(master, &salt, params)?;
        let header = VaultHeader {
            version: VAULT_VERSION,
            salt: B64.encode(salt),
            kdf: params,
            check: encrypt(&key, CHECK_PLAINTEXT)?,
        };
        Ok((header, key))
    }

    /// Dérive la clé du coffre ; un mauvais mot de passe donne `CoreError::WrongPassword`
    pub fn unlock(&self, master: &str) -> CoreResult<VaultKey> {
        if self.version > VAULT_VERSION {
            return Err(CoreError::Conflict(format!(
                "Le coffre (format v{}) provient d'une version plus récente de l'application",
                self.version
            )));
        }
        let salt = decode_b64(&self.salt, "Sel du coffre")?;
        let key = derive_key(master, &salt, self.kdf)?;
        if !self.verify(&key) {
            return Err(CoreError::WrongPassword);
        }
        Ok(key)
    }

    /// Indique si `key` est bien la clé de ce coffre
    pub fn verify(&self, key: &VaultKey) -> bool {
        matches!(
            decrypt_bytes(key, &self.check.nonce, &self.check.data),
            Ok(Some(text)) if text.as_slice() == CHECK_PLAINTEXT.as_bytes()
        )
    }
}

// ── Enveloppe d'un fichier de données chiffré ─────────────────────────────

/// Fichier de données entièrement chiffré : le JSON de l'inventaire est chiffré
/// avec la clé du coffre (même sel, même mot de passe maître)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedFile {
    pub format: String,
    pub version: u32,
    pub salt: String,
    pub kdf: KdfParams,
    pub nonce: String,
    pub data: String,
}

/// Indique si `content` se présente comme un fichier chiffré (champ `format`),
/// même si l'enveloppe est par ailleurs abîmée : il ne doit alors jamais être
/// traité comme un inventaire en clair corrompu
pub fn is_encrypted_content(content: &str) -> bool {
    #[derive(Deserialize)]
    struct Probe {
        #[serde(default)]
        format: Option<String>,
    }
    serde_json::from_str::<Probe>(content)
        .is_ok_and(|p| p.format.as_deref() == Some(ENCRYPTED_FILE_FORMAT))
}

impl EncryptedFile {
    /// Lit une enveloppe chiffrée (à n'appeler que si [`is_encrypted_content`] est vrai)
    pub fn parse(content: &str) -> CoreResult<EncryptedFile> {
        Self::detect(content)
            .ok_or_else(|| CoreError::Crypto("enveloppe du fichier chiffré invalide".into()))
    }

    /// Reconnaît une enveloppe chiffrée ; `None` pour tout autre contenu
    pub fn detect(content: &str) -> Option<EncryptedFile> {
        serde_json::from_str::<EncryptedFile>(content)
            .ok()
            .filter(|f| f.format == ENCRYPTED_FILE_FORMAT)
    }

    /// Chiffre `plaintext` avec la clé du coffre `header` (nonce neuf à chaque appel)
    pub(crate) fn seal(
        header: &VaultHeader,
        key: &VaultKey,
        plaintext: &[u8],
    ) -> CoreResult<EncryptedFile> {
        let (nonce, data) = encrypt_bytes(key, plaintext)?;
        Ok(EncryptedFile {
            format: ENCRYPTED_FILE_FORMAT.into(),
            version: ENCRYPTED_FILE_VERSION,
            salt: header.salt.clone(),
            kdf: header.kdf,
            nonce: B64.encode(nonce),
            data: B64.encode(data),
        })
    }

    /// Dérive la clé de `master` et déchiffre le contenu
    pub(crate) fn open(&self, master: &str) -> CoreResult<(Zeroizing<Vec<u8>>, VaultKey)> {
        if self.version > ENCRYPTED_FILE_VERSION {
            return Err(CoreError::Conflict(format!(
                "Ce fichier chiffré (format v{}) provient d'une version plus récente de l'application",
                self.version
            )));
        }
        let salt = decode_b64(&self.salt, "Sel du fichier")?;
        let key = derive_key(master, &salt, self.kdf)?;
        let plaintext =
            decrypt_bytes(&key, &self.nonce, &self.data)?.ok_or(CoreError::WrongPassword)?;
        Ok((plaintext, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(master: &str) -> (VaultHeader, VaultKey) {
        VaultHeader::create(master, KdfParams::insecure_for_tests()).unwrap()
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let (_, key) = header("correct horse");
        let blob = encrypt(&key, "s3cr3t é ✓").unwrap();
        assert_ne!(blob.data, "s3cr3t é ✓");
        assert_eq!(decrypt(&key, &blob).unwrap().as_str(), "s3cr3t é ✓");
        // Nonce neuf à chaque chiffrement
        let again = encrypt(&key, "s3cr3t é ✓").unwrap();
        assert_ne!(blob.nonce, again.nonce);
        assert_ne!(blob.data, again.data);
    }

    #[test]
    fn unlock_with_right_and_wrong_password() {
        let (h, key) = header("correct horse");
        let blob = encrypt(&key, "x").unwrap();
        let again = h.unlock("correct horse").unwrap();
        assert_eq!(decrypt(&again, &blob).unwrap().as_str(), "x");
        assert!(matches!(
            h.unlock("wrong horse"),
            Err(CoreError::WrongPassword)
        ));
    }

    #[test]
    fn other_key_cannot_decrypt() {
        let (_, k1) = header("password-one");
        let (h2, k2) = header("password-one");
        // Même mot de passe mais sel différent : clés différentes
        let blob = encrypt(&k1, "x").unwrap();
        assert!(decrypt(&k2, &blob).is_err());
        assert!(!h2.verify(&k1));
        assert!(h2.verify(&k2));
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let (_, key) = header("correct horse");
        let mut blob = encrypt(&key, "secret").unwrap();
        let mut raw = B64.decode(&blob.data).unwrap();
        raw[0] ^= 1;
        blob.data = B64.encode(raw);
        assert!(matches!(decrypt(&key, &blob), Err(CoreError::Crypto(_))));
        blob.nonce = "pas du base64 !".into();
        assert!(decrypt(&key, &blob).is_err());
    }

    #[test]
    fn short_master_password_is_rejected() {
        assert!(matches!(
            VaultHeader::create("court", KdfParams::insecure_for_tests()),
            Err(CoreError::Validation(_))
        ));
        // 8 caractères (et non 8 octets) suffisent
        assert!(VaultHeader::create("éééééééé", KdfParams::insecure_for_tests()).is_ok());
    }

    #[test]
    fn key_debug_hides_material() {
        let (_, key) = header("correct horse");
        assert_eq!(format!("{key:?}"), "VaultKey(***)");
    }

    #[test]
    fn encrypted_file_roundtrip_and_detection() {
        let (h, key) = header("correct horse");
        let sealed = EncryptedFile::seal(&h, &key, b"{\"vms\":[]}").unwrap();
        let json = serde_json::to_string(&sealed).unwrap();
        let found = EncryptedFile::detect(&json).expect("enveloppe reconnue");
        let (plain, file_key) = found.open("correct horse").unwrap();
        assert_eq!(plain.as_slice(), b"{\"vms\":[]}");
        assert!(h.verify(&file_key));
        assert!(matches!(
            found.open("wrong horse"),
            Err(CoreError::WrongPassword)
        ));
        assert!(EncryptedFile::detect(r#"{"version":1,"vms":[]}"#).is_none());
        assert!(is_encrypted_content(&json));
        assert!(is_encrypted_content(r#"{"format":"vm-manager-encrypted"}"#));
        assert!(!is_encrypted_content(r#"{"version":1,"vms":[]}"#));
        assert!(!is_encrypted_content("[1, 2]"));
    }

    #[test]
    fn default_params_are_production_grade() {
        let p = KdfParams::default();
        assert_eq!((p.m_cost_kib, p.t_cost, p.p_cost), (65536, 3, 1));
    }
}
