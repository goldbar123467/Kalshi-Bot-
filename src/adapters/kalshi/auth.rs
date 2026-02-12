use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::pss::SigningKey;
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use rsa::RsaPrivateKey;
use sha2::Sha256;

pub struct KalshiAuth {
    signing_key: SigningKey<Sha256>,
    pub key_id: String,
}

impl KalshiAuth {
    pub fn new(key_id: String, pem: &str) -> anyhow::Result<Self> {
        let private_key = if pem.contains("BEGIN RSA PRIVATE KEY") {
            RsaPrivateKey::from_pkcs1_pem(pem)?
        } else {
            RsaPrivateKey::from_pkcs8_pem(pem)?
        };
        Ok(Self {
            signing_key: SigningKey::<Sha256>::new(private_key),
            key_id,
        })
    }

    pub fn headers(&self, method: &str, path: &str) -> Vec<(&'static str, String)> {
        let ts = chrono::Utc::now().timestamp_millis().to_string();
        let sign_path = path.split('?').next().unwrap_or(path);
        let msg = format!("{}{}{}", ts, method, sign_path);
        let mut rng = rand::thread_rng();
        let sig = self.signing_key.sign_with_rng(&mut rng, msg.as_bytes());
        vec![
            ("KALSHI-ACCESS-KEY", self.key_id.clone()),
            ("KALSHI-ACCESS-TIMESTAMP", ts),
            ("KALSHI-ACCESS-SIGNATURE", STANDARD.encode(sig.to_bytes())),
            ("Content-Type", "application/json".into()),
        ]
    }
}
