// CA Certificate Manager
// Handles generation, loading, and export of the root CA certificate for MITM proxy

use hudsucker::rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose,
    IsCa, KeyPair, KeyUsagePurpose,
};
use std::fs;
use std::path::PathBuf;

const CA_CERT_FILENAME: &str = "quilr_proxy_ca.crt";
const CA_KEY_FILENAME: &str = "quilr_proxy_ca.key";

/// Get the directory where CA files are stored
pub fn get_ca_dir() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("quilr-agent-gateway");

    // Create directory if it doesn't exist
    let _ = fs::create_dir_all(&config_dir);
    config_dir
}

/// Get path to CA certificate file
pub fn get_ca_cert_path() -> PathBuf {
    get_ca_dir().join(CA_CERT_FILENAME)
}

/// Get path to CA private key file
pub fn get_ca_key_path() -> PathBuf {
    get_ca_dir().join(CA_KEY_FILENAME)
}

/// Check if CA certificate exists
pub fn ca_exists() -> bool {
    get_ca_cert_path().exists() && get_ca_key_path().exists()
}

/// Generate a new CA certificate and private key
pub fn generate_ca() -> Result<(String, String), String> {
    println!("[CA] Generating new CA certificate...");

    // Generate a new key pair
    let key_pair = KeyPair::generate().map_err(|e| format!("Failed to generate key pair: {}", e))?;

    // Set up certificate parameters
    let mut params = CertificateParams::default();

    // Set distinguished name
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Quilr Agent Gateway CA");
    dn.push(DnType::OrganizationName, "Quilr");
    dn.push(DnType::OrganizationalUnitName, "Agent Gateway");
    params.distinguished_name = dn;

    // Make it a CA certificate
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ServerAuth,
        ExtendedKeyUsagePurpose::ClientAuth,
    ];

    // Set validity period (10 years)
    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after = time::OffsetDateTime::now_utc() + time::Duration::days(3650);

    // Generate the certificate
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("Failed to generate certificate: {}", e))?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    // Save to files
    let cert_path = get_ca_cert_path();
    let key_path = get_ca_key_path();

    fs::write(&cert_path, &cert_pem)
        .map_err(|e| format!("Failed to write CA certificate: {}", e))?;
    fs::write(&key_path, &key_pem)
        .map_err(|e| format!("Failed to write CA private key: {}", e))?;

    // Set restrictive permissions on key file (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600));
    }

    println!("[CA] CA certificate generated and saved to: {:?}", cert_path);
    println!("[CA] CA private key saved to: {:?}", key_path);

    Ok((cert_pem, key_pem))
}

/// Load existing CA certificate and key from files
pub fn load_ca() -> Result<(String, String), String> {
    let cert_path = get_ca_cert_path();
    let key_path = get_ca_key_path();

    let cert_pem = fs::read_to_string(&cert_path)
        .map_err(|e| format!("Failed to read CA certificate: {}", e))?;
    let key_pem = fs::read_to_string(&key_path)
        .map_err(|e| format!("Failed to read CA private key: {}", e))?;

    println!("[CA] Loaded CA certificate from: {:?}", cert_path);

    Ok((cert_pem, key_pem))
}

/// Get or generate CA certificate (loads if exists, generates if not)
pub fn get_or_generate_ca() -> Result<(String, String), String> {
    if ca_exists() {
        load_ca()
    } else {
        generate_ca()
    }
}

/// Export CA certificate to a specified path (for user to install)
pub fn export_ca_cert(dest_path: &str) -> Result<(), String> {
    let cert_path = get_ca_cert_path();

    if !cert_path.exists() {
        return Err("CA certificate does not exist. Start the proxy first to generate it.".to_string());
    }

    fs::copy(&cert_path, dest_path)
        .map_err(|e| format!("Failed to export CA certificate: {}", e))?;

    println!("[CA] Exported CA certificate to: {}", dest_path);
    Ok(())
}

/// Get CA certificate content as string (for display or export)
pub fn get_ca_cert_content() -> Result<String, String> {
    let cert_path = get_ca_cert_path();

    if !cert_path.exists() {
        return Err("CA certificate does not exist. Start the proxy first to generate it.".to_string());
    }

    fs::read_to_string(&cert_path)
        .map_err(|e| format!("Failed to read CA certificate: {}", e))
}

/// Get CA certificate path as string
pub fn get_ca_cert_path_string() -> String {
    get_ca_cert_path().to_string_lossy().to_string()
}
