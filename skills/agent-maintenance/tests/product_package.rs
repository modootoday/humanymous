//! Real staged artifact test, for approved hosted execution only.
use humanymous_agent_maintenance::package;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

#[test]
fn product_bundle_contains_paired_suite_and_refuses_existing_output() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let output = std::env::temp_dir().join(format!("humanymous-product-bundle-{}-{}",
        std::process::id(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let receipt = package::stage(root, &output, "product").unwrap();
    assert!(!receipt.runtime_qualified);
    assert!(!receipt.release_qualified);
    for entry in &receipt.files {
        let bytes = fs::read(output.join(&entry.path)).unwrap();
        assert_eq!(bytes.len(), entry.bytes);
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), entry.sha256);
        assert!(!entry.path.contains("humanymous-development"));
        assert!(!entry.path.starts_with(".agents/"));
    }
    let cases = receipt.files.iter().filter(|entry|
        entry.path.starts_with("plugins/humanymous-product/evals/")
            && entry.path.ends_with("/prompt.md")).count();
    assert_eq!(cases, 3);
    let before = fs::read(output.join("bundle-receipt.json")).unwrap();
    assert!(package::stage(root, &output, "product").is_err());
    assert_eq!(fs::read(output.join("bundle-receipt.json")).unwrap(), before);
    fs::remove_dir_all(output).unwrap();
}
