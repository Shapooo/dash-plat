use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use ed25519::pkcs8::{DecodePrivateKey, EncodePrivateKey, KeypairBytes};
use ed25519_dalek::{PublicKey, SecretKey};
use hotstuff_rs::types::{DalekKeypair, PublicKeyBytes};
use rand::rngs::OsRng;

pub fn generate_keypair() -> DalekKeypair {
    let mut rng = OsRng {};
    ed25519_dalek::Keypair::generate(&mut rng)
}

pub fn keypair_to_pem(keypair: DalekKeypair) -> String {
    let kpb = keypair_to_bytes(keypair);
    kpb.to_pkcs8_pem(pkcs8::LineEnding::LF).unwrap().to_string()
}

pub fn keypair_from_pem(pem: &str) -> Result<DalekKeypair> {
    let kpb = KeypairBytes::from_pkcs8_pem(pem).unwrap();
    keypair_from_bytes(kpb)
}

pub fn publickey_to_base64(pubkey: PublicKeyBytes) -> String {
    general_purpose::STANDARD.encode(pubkey)
}

pub fn publickey_from_base64(b64: &str) -> Result<PublicKeyBytes> {
    let key_vec = general_purpose::STANDARD.decode(b64)?;
    Ok(key_vec.as_slice().try_into()?)
}

pub fn keypair_to_bytes(keypair: DalekKeypair) -> KeypairBytes {
    KeypairBytes {
        secret_key: keypair.secret.to_bytes(),
        public_key: Some(keypair.public.to_bytes()),
    }
}

pub fn keypair_from_bytes(kpb: KeypairBytes) -> Result<DalekKeypair> {
    let secret = SecretKey::from_bytes(&kpb.secret_key)?;
    let public = match kpb.public_key {
        Some(pubkey) => PublicKey::from_bytes(&pubkey).unwrap(),
        None => (&secret).into(),
    };
    Ok(DalekKeypair { secret, public })
}

#[cfg(test)]
mod crypto_tests {
    use super::*;

    #[test]
    fn basic_test() {
        let keypair = generate_keypair();
        let pubkey_bytes = keypair.public.to_bytes();
        let sk_pem = keypair_to_pem(keypair);
        let pk_b64 = publickey_to_base64(pubkey_bytes);
        let pubkeybytes = publickey_from_base64(&pk_b64).unwrap();
        let keypair_parsed = keypair_from_pem(&sk_pem).unwrap();
        assert_eq!(keypair_parsed.public.to_bytes(), pubkeybytes);
    }
}
