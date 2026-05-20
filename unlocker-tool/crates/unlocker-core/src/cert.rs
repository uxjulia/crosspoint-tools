use anyhow::Result;
use rcgen::{CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, KeyPair};

/// A self-signed TLS certificate for the spoofed hostnames.
pub struct SelfSignedCert {
    pub cert_pem: String,
    pub key_pem: String,
}

/// Generate a self-signed certificate valid for the given hostnames.
/// Used by the helper to serve HTTPS on port 443 for the spoofed API domains.
pub fn generate_self_signed(hostnames: &[&str]) -> Result<SelfSignedCert> {
    let sans: Vec<String> = hostnames.iter().map(|s| (*s).to_string()).collect();
    let mut params = CertificateParams::new(sans)?;
    let mut dn = DistinguishedName::new();
    dn.push(
        DnType::CommonName,
        hostnames.first().copied().unwrap_or("localhost"),
    );
    dn.push(DnType::OrganizationName, "CrossPoint Reader");
    params.distinguished_name = dn;
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    let key = KeyPair::generate()?;
    let cert = params.self_signed(&key)?;
    Ok(SelfSignedCert {
        cert_pem: cert.pem(),
        key_pem: key.serialize_pem(),
    })
}
