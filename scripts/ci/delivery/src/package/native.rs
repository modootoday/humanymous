use crate::{Result, require, source, write_json};
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path};

pub(super) fn writer<W: Write>(output: W) -> tar::Builder<W> {
    let mut archive = tar::Builder::new(output);
    archive.sparse(false);
    archive.follow_symlinks(false);
    archive
}

pub fn native_artifacts(root: &Path) -> Result<()> {
    let summary = native_archive(root, None)?;
    crate::evidence::emit_public_summary("Native component archive", &summary)
}
pub fn native_artifacts_bound(root: &Path, build: &Value) -> Result<Value> {
    native_archive(root, Some(build))
}
fn native_archive(root: &Path, build: Option<&Value>) -> Result<Value> {
    // Actual names are canonical in runtime/CLI INTEGRATION. These are bounded
    // components, never invented server/gate aliases or release qualification.
    let names = ["humanymous", "humanymous-runtime"];
    let before = source::snapshot(root)?;
    if let Some(build) = build {
        require(
            build["schema"] == "humanymous.native-build-receipt/1"
                && build["build_finished"] == true
                && build["source_sha"] == before["source_sha"]
                && build["source_manifest_sha256"] == before["manifest_sha256"]
                && build["binaries"]
                    .as_object()
                    .is_some_and(|rows| rows.len() == 2),
            "native build receipt/source mismatch",
        )?;
    }
    let mut members = crate::notices::members::Members::new();
    for binary in names {
        let filename = format!("{binary}{}", std::env::consts::EXE_SUFFIX);
        let path = root.join("target/release").join(&filename);
        let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        require(
            meta.is_file() && !meta.file_type().is_symlink() && meta.len() > 0,
            "native binary not a regular nonempty file",
        )?;
        let hash = source::file(&path)?;
        if let Some(build) = build {
            require(
                build["binaries"][binary]["sha256"] == hash
                    && build["binaries"][binary]["bytes"].as_u64() == Some(meta.len()),
                "produced native binary changed",
            )?;
        }
        members.insert(filename, (meta.len(), hash));
    }
    for (input, name) in [
        ("LICENSE", "LICENSE"),
        (
            "crates/sdk/humanymous-runtime/src/audit/LICENSE-Go.txt",
            "licenses/Go.txt",
        ),
    ] {
        let path = root.join(input);
        members.insert(
            name.into(),
            (
                fs::metadata(&path).map_err(|e| e.to_string())?.len(),
                source::file(&path)?,
            ),
        );
    }
    let directory = root.join("target/delivery-artifacts");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let head = source::head(root)?;
    let name = format!(
        "humanymous-components-{}-{}-{head}.tar.gz",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let file = fs::File::create(directory.join(&name)).map_err(|e| e.to_string())?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = writer(encoder);
    for binary in names {
        let filename = format!("{binary}{}", std::env::consts::EXE_SUFFIX);
        archive
            .append_path_with_name(root.join("target/release").join(&filename), filename)
            .map_err(|e| e.to_string())?;
    }
    archive
        .append_path_with_name(root.join("LICENSE"), "LICENSE")
        .map_err(|e| e.to_string())?;
    let notices = crate::notices::append(root, &mut archive)?;
    archive
        .into_inner()
        .map_err(|e| e.to_string())?
        .finish()
        .map_err(|e| e.to_string())?;
    let notice_verification = crate::notices::verify_archive(&directory.join(&name))?;
    let content_verification = match crate::notices::members::verify(
        &directory.join(&name),
        &members,
    ) {
        Ok(proof) => proof,
        Err(failure) => {
            let summary = json!({"schema":"humanymous.native-archive-summary/1","status":"failed",
                "source_sha":before["source_sha"],"source_manifest_sha256":before["manifest_sha256"],
                "archive_sha256":source::file(&directory.join(&name)).ok(),
                "known_notice_verification":notice_verification,"content_verification":failure.public(),
                "binary_build_source_receipt_verified":build.is_some(),
                "source_changed":source::snapshot(root).ok().map(|after| after != before),
                "full_product_qualified":false,"release_ready":false});
            let _ = crate::evidence::emit_public_summary("Native archive member refusal", &summary);
            return Err("native archive member verification failed".into());
        }
    };
    require(
        source::snapshot(root)? == before,
        "native packaging source changed",
    )?;
    for binary in names {
        let filename = format!("{binary}{}", std::env::consts::EXE_SUFFIX);
        require(
            source::file(&root.join("target/release").join(&filename))? == members[&filename].1,
            "native binary changed during packaging",
        )?;
    }
    let archive_hash = source::file(&directory.join(&name))?;
    write_json(
        &directory.join(format!("{name}.manifest.json")),
        &json!({"schema":"humanymous.native-components/1","source":before,"archive_sha256":archive_hash,"known_distribution_notices":notices,"known_notice_verification":notice_verification,"content_verification":content_verification,"binary_build_source_receipt_verified":build.is_some(),"build_receipt":build,"full_product_qualified":false,"release_ready":false}),
    )?;
    Ok(
        json!({"schema":"humanymous.native-archive-summary/1","source_sha":before["source_sha"],
        "source_manifest_sha256":before["manifest_sha256"],"archive_sha256":archive_hash,
        "binary_build_source_receipt_verified":build.is_some(),"content_verification":content_verification,
        "known_notice_verification":notice_verification,"source_changed":false,
        "full_product_qualified":false,"release_ready":false}),
    )
}

#[cfg(test)]
mod tests;
