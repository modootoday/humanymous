use super::{ServiceSecretReference, unavailable, validate_secret};
use keyring_core::api::CredentialStoreApi;
use std::{collections::HashMap, io};
use windows_native_keyring_store::Store;
use zeroize::Zeroizing;

pub(super) fn attributes_match(attributes: &HashMap<String, String>, target: &str) -> bool {
    attributes.get("target_name").map(String::as_str) == Some(target)
        && attributes.get("persistence").map(String::as_str) == Some("Local")
}

pub(super) fn entry(reference: &ServiceSecretReference) -> io::Result<keyring_core::Entry> {
    // Credential Manager is scoped to the current token user. Require the
    // actual virtual service principal, never the installer's credential store.
    let expected =
        crate::storage::resolve_service_sid(reference.service_name()).map_err(|_| unavailable())?;
    let actual = crate::process::current_identity().map_err(|_| unavailable())?;
    if actual.sid() != expected {
        return Err(unavailable());
    }
    let target = reference.target();
    // Explicit native store avoids process-global default or mock backends.
    let store = Store::new().map_err(|_| unavailable())?;
    store
        .build(
            "humanymous-service",
            reference.service_name(),
            Some(&HashMap::from([
                ("target", target.as_str()),
                ("persistence", "Local"),
            ])),
        )
        .map_err(|_| unavailable())
}

pub(super) fn read(reference: &ServiceSecretReference) -> io::Result<Zeroizing<Vec<u8>>> {
    let entry = entry(reference)?;
    let target = reference.target();
    let before = entry.get_attributes().map_err(|_| unavailable())?;
    if !attributes_match(&before, &target) {
        return Err(unavailable());
    }
    let secret = Zeroizing::new(entry.get_secret().map_err(|_| unavailable())?);
    validate_secret(&secret)?;
    let after = entry.get_attributes().map_err(|_| unavailable())?;
    if !attributes_match(&after, &target) {
        return Err(unavailable());
    }
    // These reads are not an atomic anti-ABA credential snapshot. They constrain
    // the selected store/target/persistence; sealed custody authenticates keys.
    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_exact_target_and_existing_local_persistence_are_accepted() {
        let target = "humanymous/service/humanymous-lab-test/audit";
        let good = HashMap::from([
            ("target_name".to_owned(), target.to_owned()),
            ("persistence".to_owned(), "Local".to_owned()),
        ]);
        assert!(attributes_match(&good, target));
        for persistence in ["Session", "Enterprise", "local", ""] {
            let mut bad = good.clone();
            bad.insert("persistence".to_owned(), persistence.to_owned());
            assert!(!attributes_match(&bad, target));
        }
        assert!(!attributes_match(&good, "other"));
        assert!(!attributes_match(&HashMap::new(), target));
    }
}
