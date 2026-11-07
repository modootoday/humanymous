use super::*;
use std::{
    io::{Seek, SeekFrom},
    path::PathBuf,
};

struct OwnedFiles(Vec<PathBuf>);
impl OwnedFiles {
    fn create(&mut self, path: &Path) -> fs::File {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        self.0.push(path.to_owned());
        file
    }
    fn cleanup(&mut self) {
        while let Some(path) = self.0.pop() {
            fs::remove_file(path).unwrap();
        }
    }
}
impl Drop for OwnedFiles {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

#[test]
fn canonical_writer_preserves_owned_file_and_hardlink_bytes_as_regular_members() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!(
        "humanymous-native-archive-{}-{nonce}",
        std::process::id()
    ));
    let input = base.with_extension("input");
    let link = base.with_extension("link");
    let output = base.with_extension("tar.gz");
    let mut owned = OwnedFiles(Vec::new());
    let mut file = owned.create(&input);
    file.set_len(1024 * 1024).unwrap();
    file.seek(SeekFrom::End(-3)).unwrap();
    file.write_all(b"end").unwrap();
    file.flush().unwrap();
    drop(file);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = fs::metadata(&input).unwrap();
        assert!(
            meta.blocks() * 512 < meta.len(),
            "Unix fixture must contain an observed sparse region"
        );
    }
    fs::hard_link(&input, &link).unwrap();
    owned.0.push(link.clone());
    let encoder =
        flate2::write::GzEncoder::new(owned.create(&output), flate2::Compression::default());
    let mut archive = writer(encoder);
    archive.append_path_with_name(&input, "humanymous").unwrap();
    archive
        .append_path_with_name(&link, "humanymous-runtime")
        .unwrap();
    archive
        .append_path_with_name(root.join("LICENSE"), "LICENSE")
        .unwrap();
    crate::notices::append(&root, &mut archive).unwrap();
    archive.into_inner().unwrap().finish().unwrap();
    let mut expected = crate::notices::members::Members::new();
    let license = root.join("LICENSE");
    let notice = root.join("crates/sdk/humanymous-runtime/src/audit/LICENSE-Go.txt");
    for (path, name) in [
        (&input, "humanymous"),
        (&link, "humanymous-runtime"),
        (&license, "LICENSE"),
        (&notice, "licenses/Go.txt"),
    ] {
        expected.insert(
            name.into(),
            (
                fs::metadata(path).unwrap().len(),
                source::file(path).unwrap(),
            ),
        );
    }
    assert_eq!(
        crate::notices::members::verify(&output, &expected).unwrap()["status"],
        "verified"
    );
    assert_eq!(
        crate::notices::verify_archive(&output).unwrap()["status"],
        "verified"
    );
    owned.cleanup();
}
