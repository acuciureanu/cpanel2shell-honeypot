//! On-demand certificate generation using `rcgen`.

use rcgen::{CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, KeyUsagePurpose};
use time::{Duration, OffsetDateTime};

use super::config::{AutoSettings, HostEntry};

pub struct GeneratedCert {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
}

pub fn generate_for_sni(
    sni: &str,
    entry: Option<&HostEntry>,
    auto: &AutoSettings,
) -> GeneratedCert {
    let validity_days = entry
        .and_then(|e| e.validity_days)
        .unwrap_or(auto.validity_days);
    let key_type_str = entry
        .and_then(|e| e.key_type.as_deref())
        .unwrap_or(&auto.key_type)
        .to_string();

    // Collect SANs
    let mut san_hosts: Vec<String> = vec![sni.to_string()];
    if let Some(e) = entry {
        for s in &e.sans {
            if !san_hosts.contains(s) {
                san_hosts.push(s.clone());
            }
        }
    }

    let key_pair = make_keypair(&key_type_str);

    let mut params = CertificateParams::new(san_hosts).expect("valid SANs");

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, sni);
    dn.push(DnType::OrganizationName, "cPanel Hosting Services");
    dn.push(DnType::CountryName, "US");
    params.distinguished_name = dn;

    // Validity: backdate 30 days, expire validity_days from now.
    let not_before = if let Some(nb) = entry.and_then(|e| e.not_before.as_deref()) {
        parse_not_before(nb)
    } else {
        OffsetDateTime::now_utc() - Duration::days(30)
    };
    let not_after = OffsetDateTime::now_utc() + Duration::days(validity_days as i64);

    params.not_before = not_before;
    params.not_after = not_after;
    params.is_ca = IsCa::ExplicitNoCa;
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];

    let cert = params
        .self_signed(&key_pair)
        .expect("cert generation failed");

    GeneratedCert {
        cert_pem: cert.pem().into_bytes(),
        key_pem: key_pair.serialize_pem().into_bytes(),
    }
}

pub fn generate_simple(hostnames: Vec<String>) -> GeneratedCert {
    let key_pair = make_keypair("ecdsa-p256");
    let mut params = CertificateParams::new(hostnames).expect("valid SANs");
    params.not_before = OffsetDateTime::now_utc() - Duration::days(30);
    params.not_after = OffsetDateTime::now_utc() + Duration::days(365);
    let cert = params
        .self_signed(&key_pair)
        .expect("cert generation failed");
    GeneratedCert {
        cert_pem: cert.pem().into_bytes(),
        key_pem: key_pair.serialize_pem().into_bytes(),
    }
}

fn make_keypair(key_type: &str) -> KeyPair {
    match key_type {
        "ecdsa-p256" => {
            KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).expect("ecdsa-p256 keygen failed")
        }
        "ed25519" => KeyPair::generate_for(&rcgen::PKCS_ED25519).expect("ed25519 keygen failed"),
        // rsa-2048/rsa-4096 fall back to ecdsa-p256 (rcgen requires ring for RSA)
        _ => KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).expect("keygen failed"),
    }
}

fn parse_not_before(s: &str) -> OffsetDateTime {
    // Minimal ISO date parsing: "YYYY-MM-DD" or "YYYY-MM-DDTHH:MM:SSZ"
    let parts: Vec<i64> = s
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse().ok())
        .collect();
    if parts.len() >= 3 {
        let year = parts[0] as i32;
        let month = (parts[1] as u8).clamp(1, 12);
        let day = (parts[2] as u8).clamp(1, 28);
        use time::Month;
        let m = Month::try_from(month).unwrap_or(Month::January);
        time::Date::from_calendar_date(year, m, day)
            .map(|d| d.with_hms(0, 0, 0).unwrap().assume_utc())
            .unwrap_or_else(|_| OffsetDateTime::now_utc() - Duration::days(30))
    } else {
        OffsetDateTime::now_utc() - Duration::days(30)
    }
}
