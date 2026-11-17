//! Cryptographic identity and signatures for BabelForge.
//!
//! Ed25519 for node identity. No blockchain, no wallets, no token economics.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use forge_contracts::ForgeError;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct NodeKeyPair {
    signing: SigningKey,
    verifying: VerifyingKey,
}

impl NodeKeyPair {
    pub fn generate() -> Self {
        let signing = SigningKey::generate(&mut OsRng);
        let verifying = signing.verifying_key();
        Self { signing, verifying }
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.verifying.as_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> String {
        let sig = self.signing.sign(message);
        hex::encode(sig.to_bytes())
    }

    pub fn verify(public_key_hex: &str, message: &[u8], signature_hex: &str) -> Result<(), ForgeError> {
        let pk_bytes = hex::decode(public_key_hex)
            .map_err(|e| ForgeError::new("invalid_key", e.to_string()))?;
        let pk_array: [u8; 32] = pk_bytes
            .try_into()
            .map_err(|_| ForgeError::new("invalid_key", "expected 32-byte public key"))?;
        let vk = VerifyingKey::from_bytes(&pk_array)
            .map_err(|e| ForgeError::new("invalid_key", e.to_string()))?;

        let sig_bytes = hex::decode(signature_hex)
            .map_err(|e| ForgeError::new("invalid_signature", e.to_string()))?;
        let sig_array: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| ForgeError::new("invalid_signature", "expected 64-byte signature"))?;
        let sig = Signature::from_bytes(&sig_array);

        vk.verify(message, &sig)
            .map_err(|e| ForgeError::new("verification_failed", e.to_string()))
    }
}

pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPayload {
    pub payload_hash: String,
    pub signature_hex: String,
    pub public_key_hex: String,
}

impl SignedPayload {
    pub fn sign(keypair: &NodeKeyPair, payload: &[u8]) -> Self {
        let payload_hash = hash_bytes(payload);
        let signature_hex = keypair.sign(payload_hash.as_bytes());
        Self {
            payload_hash,
            signature_hex,
            public_key_hex: keypair.public_key_hex(),
        }
    }

    pub fn verify(&self) -> Result<(), ForgeError> {
        NodeKeyPair::verify(
            &self.public_key_hex,
            self.payload_hash.as_bytes(),
            &self.signature_hex,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify() {
        let kp = NodeKeyPair::generate();
        let msg = b"hello babel";
        let sig = kp.sign(msg);
        NodeKeyPair::verify(&kp.public_key_hex(), msg, &sig).unwrap();
    }

    #[test]
    fn signed_payload_roundtrip() {
        let kp = NodeKeyPair::generate();
        let sp = SignedPayload::sign(&kp, b"payload");
        sp.verify().unwrap();
    }
}
