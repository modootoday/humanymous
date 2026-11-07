//! Build metadata through upstream RubyGems APIs; no authored Ruby source.
use magnus::{Error, RClass, RHash, RModule, Ruby, Value, prelude::*};
use serde::Deserialize;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    staging: String,
    metadata: serde_json::Value,
}

pub fn from_private_request(ruby: &Ruby) -> Result<(), Error> {
    let Some(path) = std::env::var_os("HUMANYMOUS_NATIVE_GEM_PACKAGE_REQUEST") else {
        return Ok(());
    };
    let error = |s: &str| Error::new(ruby.exception_runtime_error(), s);
    let file =
        std::fs::File::open(path).map_err(|_| error("native gem package request unavailable"))?;
    let mut data = Vec::new();
    file.take(65537)
        .read_to_end(&mut data)
        .map_err(|_| error("native gem package request unreadable"))?;
    if data.len() > 65536 {
        return Err(error("native gem package request exceeds limit"));
    }
    let request: Request =
        serde_json::from_slice(&data).map_err(|_| error("invalid native gem package request"))?;
    let artifact = build(ruby, request.staging, request.metadata.to_string())?;
    println!("humanymous-native-gem-artifact={artifact}");
    ruby.require("digest")?;
    let digest: RModule = ruby.class_object().const_get("Digest")?;
    let sha256: RClass = digest.const_get("SHA256")?;
    let digest: Value = sha256.funcall("file", (artifact.clone(),))?;
    let hash: String = digest.funcall("hexdigest", ())?;
    let gem: RModule = ruby.class_object().const_get("Gem")?;
    let generator_version: String = gem.const_get("VERSION")?;
    println!(
        "{}",
        serde_json::json!({"schema":"humanymous.native-gem-build/1","artifact":artifact,
        "sha256":hash,"generator":"RubyGems::Package","generatorVersion":generator_version,
        "status":"archive-built-only","wholeProductQualified":false,"actualFrameworkQualified":false})
    );
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Metadata {
    name: String,
    version: String,
    summary: String,
    authors: Vec<String>,
    license: String,
    homepage: String,
}
pub fn build(ruby: &Ruby, staging: String, metadata_json: String) -> Result<String, Error> {
    let error = |s: &str| Error::new(ruby.exception_runtime_error(), s);
    let m: Metadata =
        serde_json::from_str(&metadata_json).map_err(|_| error("invalid gem metadata"))?;
    if m.name.is_empty()
        || !m
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        || m.authors.is_empty()
        || m.version.is_empty()
    {
        return Err(error("incomplete gem metadata"));
    }
    let root = Path::new(&staging)
        .canonicalize()
        .map_err(|_| error("gem staging unavailable"))?;
    let pwd = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .map_err(|_| error("working directory unavailable"))?;
    if root != pwd {
        return Err(error("run packager in the explicit staging directory"));
    }
    ruby.require("rubygems/package")?;
    ruby.require("rbconfig")?;
    let rbconfig: RModule = ruby.class_object().const_get("RbConfig")?;
    let config: RHash = rbconfig.const_get("CONFIG")?;
    let ext: String = config.fetch("DLEXT")?;
    let abi: String = config.fetch("ruby_version")?;
    let version: String = ruby.class_object().const_get("RUBY_VERSION")?;
    let engine: String = ruby.class_object().const_get("RUBY_ENGINE")?;
    if engine != "ruby" {
        return Err(error("only CRuby native ABI is supported"));
    }
    let mut parts = version.split('.');
    let major = parts
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| error("unknown Ruby ABI"))?;
    let minor = parts
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| error("unknown Ruby ABI"))?;
    let filename = format!("lib/humanymous_ruby.{ext}");
    if !root.join(&filename).is_file()
        || !root.join("README.md").is_file()
        || !root.join("LICENSE").is_file()
    {
        return Err(error("native binary README and LICENSE required"));
    }
    let gem: RModule = ruby.class_object().const_get("Gem")?;
    let spec_class: RClass = gem.const_get("Specification")?;
    let spec: Value = spec_class.new_instance(())?;
    spec.funcall::<_, _, Value>("name=", (m.name,))?;
    spec.funcall::<_, _, Value>("version=", (m.version,))?;
    spec.funcall::<_, _, Value>("summary=", (m.summary,))?;
    spec.funcall::<_, _, Value>("authors=", (m.authors,))?;
    spec.funcall::<_, _, Value>("license=", (m.license,))?;
    spec.funcall::<_, _, Value>("homepage=", (m.homepage,))?;
    spec.funcall::<_, _, Value>(
        "files=",
        (vec![filename, "README.md".into(), "LICENSE".into()],),
    )?;
    spec.funcall::<_, _, Value>("require_paths=", (vec!["lib"],))?;
    let platform: RClass = gem.const_get("Platform")?;
    let local: Value = platform.funcall("local", ())?;
    spec.funcall::<_, _, Value>("platform=", (local,))?;
    // Single ABI artifact: do not imply platform tag selects Ruby minor ABI.
    let requirement: RClass = gem.const_get("Requirement")?;
    let requirement: Value = requirement.new_instance((
        format!(">={major}.{minor}.0"),
        format!("<{major}.{}.0", minor + 1),
    ))?;
    spec.funcall::<_, _, Value>("required_ruby_version=", (requirement,))?;
    let metadata = ruby.hash_new();
    metadata.aset("humanymous_ruby_abi", abi)?;
    metadata.aset("humanymous_qualification", "not_run")?;
    spec.funcall::<_, _, Value>("metadata=", (metadata,))?;
    let package: RClass = gem.const_get("Package")?;
    package.funcall("build", (spec,))
}
