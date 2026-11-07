//! Pure privacy admission. Expected source/run/job inputs must be supplied by
//! the later independent Git/API binding layer; these values are not attestation.
use super::{Bundle, FILES, Limits, fields};
use crate::{Result, require, source};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const HASH_MEANING: &str = "files SHA256 hashes actual executed working bytes; canonical Git object IDs identify stored Git bytes and are not interchangeable hashes";
const PROFILE_BYTES: &[u8] =
    include_bytes!("../../../../../adapters/native/header-proof-profile.json");

pub(super) struct Profile {
    value: Value,
    sha256: String,
    paths: Vec<String>,
}
pub(super) struct Input {
    pub git_object_id: String,
    pub bytes: u64,
    pub sha256: String,
}
pub(super) struct Expected {
    pub source_sha: String,
    pub run_id: u64,
    pub run_attempt: u64,
    pub job_name: String,
    pub canonical_tree_sha256: String,
    pub inputs: BTreeMap<String, Input>,
}
fn strict(bytes: &[u8], cap: usize) -> Result<Value> {
    humanymous_platform::testing::json::decode_strict_json(bytes, cap)
        .map_err(|_| "header admission JSON refused".into())
}
fn digest(value: &Value) -> Result<()> {
    require(
        crate::lab_receipts::digest(value, 64),
        "header admission digest refused",
    )
}
fn expected_path(id: usize) -> String {
    match id {
        0 => "native-abi-v1.h".into(),
        1 => "header-compiler/scope.txt".into(),
        14 => "source-before.json".into(),
        15 => "source-after.json".into(),
        _ => {
            let language = if id < 8 { "c" } else { "cpp" };
            let suffix = [
                "version",
                "version-stderr",
                "target",
                "target-stderr",
                "stdout",
                "stderr",
            ][(id - 2) % 6];
            format!("header-compiler/{language}-{suffix}.txt")
        }
    }
}
impl Profile {
    pub(super) fn compiled() -> Result<Self> {
        Self::parse(PROFILE_BYTES)
    }
    fn parse(bytes: &[u8]) -> Result<Self> {
        let value = strict(bytes, 64 * 1024)?;
        fields(
            &value,
            &[
                "schema",
                "profile",
                "admission",
                "observationEvidence",
                "observationVenue",
                "hostedQualified",
                "dynamicAbiQualified",
                "toolProvenance",
                "image",
                "platform",
                "compiler",
                "header",
                "layoutOutputs",
                "requireEmptyStderr",
                "ownerSuccessMarker",
                "ownerScopeText",
                "unknownOutput",
                "sourceSnapshots",
                "fileIdsInOrder",
                "limits",
                "acceptance",
            ],
        )?;
        require(
            value["schema"] == "humanymous.header-proof-profile/1"
                && value["profile"] == "clang18-bookworm-amd64-r3-exact-bytes"
                && value["admission"]
                    == "candidate-requires-integration-review-and-delivery-validation"
                && value["hostedQualified"] == false
                && value["dynamicAbiQualified"] == false
                && value["platform"] == "linux/amd64"
                && value["requireEmptyStderr"] == true
                && value["unknownOutput"] == "retention-incomplete-no-payload-export",
            "header profile identity refused",
        )?;
        fields(
            &value["compiler"],
            &[
                "cCommand",
                "cppCommand",
                "resolvedPath",
                "sha256",
                "requireUnchangedAfter",
                "target",
                "versionBytes",
                "versionSha256",
                "targetBytes",
                "targetSha256",
            ],
        )?;
        fields(
            &value["header"],
            &["generator", "bytes", "sha256", "owningVerifier"],
        )?;
        fields(&value["layoutOutputs"], &["c", "cpp"])?;
        fields(
            &value["limits"],
            &[
                "headerBytes",
                "sourceSnapshotBytesEach",
                "otherFileBytesEach",
                "totalDecodedBytes",
                "decodedChunkBytes",
                "retainedLogBytes",
                "protocolLineBytes",
            ],
        )?;
        fields(
            &value["sourceSnapshots"],
            &[
                "schema",
                "requireOriginalBytes",
                "requireCleanExactSourceAndUnchanged",
                "requireCandidateCanonicalPublicPathSet",
                "candidatePathCountAndSizeApproval",
            ],
        )?;
        fields(
            &value["acceptance"],
            &[
                "ownerSemanticLayoutChecksStillRequired",
                "requireOriginalByteRoundTripAndDigests",
                "requireAllFilesValidatedBeforeFirstChunk",
                "requireOneCompleteEnvelopeBoundToOfficialRunAndJob",
                "requireFreshPrivateReconstructionRoot",
                "partialOrUnknownNeverCompletesProof",
                "profileDoesNotAuthorizeDispatchOrPublication",
            ],
        )?;
        require(
            value["acceptance"]
                .as_object()
                .is_some_and(|rows| rows.values().all(|flag| flag == true))
                && value["sourceSnapshots"]["schema"] == "humanymous.delivery-source/1"
                && value["sourceSnapshots"]["requireOriginalBytes"] == true
                && value["sourceSnapshots"]["requireCleanExactSourceAndUnchanged"] == true
                && value["sourceSnapshots"]["requireCandidateCanonicalPublicPathSet"] == true
                && value["sourceSnapshots"]["candidatePathCountAndSizeApproval"]
                    == "required-before-hosted-dispatch",
            "header profile acceptance contract refused",
        )?;
        for language in ["c", "cpp"] {
            fields(
                &value["layoutOutputs"][language],
                &["standard", "bytes", "sha256", "lines", "rootRecords"],
            )?;
            digest(&value["layoutOutputs"][language]["sha256"])?;
        }
        for key in ["sha256", "versionSha256", "targetSha256"] {
            digest(&value["compiler"][key])?;
        }
        digest(&value["header"]["sha256"])?;
        require(
            value["compiler"]["cCommand"] == "clang"
                && value["compiler"]["cppCommand"] == "clang++"
                && value["compiler"]["requireUnchangedAfter"] == true,
            "header compiler recipe refused",
        )?;
        let rows = value["fileIdsInOrder"]
            .as_array()
            .ok_or("header profile file table absent")?;
        require(
            rows.len() == FILES,
            "header profile file table count refused",
        )?;
        let mut paths = Vec::new();
        for (id, row) in rows.iter().enumerate() {
            fields(row, &["id", "path"])?;
            let path = expected_path(id);
            require(
                row["id"] == id && row["path"] == path && source::safe_relative(&path),
                "header profile semantic file mapping refused",
            )?;
            paths.push(path);
        }
        Limits::profile(&value)?;
        Ok(Self {
            value,
            sha256: source::sha(bytes),
            paths,
        })
    }
    pub(super) fn paths(&self) -> &[String] {
        &self.paths
    }
    pub(super) fn value(&self) -> &Value {
        &self.value
    }
    pub(super) fn sha256(&self) -> &str {
        &self.sha256
    }
    pub(super) fn limits(&self) -> Result<Limits> {
        Limits::profile(&self.value)
    }
}

pub(super) fn snapshot(bytes: &[u8], expected: &Expected, cap: usize) -> Result<()> {
    let value = strict(bytes, cap)?;
    fields(
        &value,
        &[
            "schema",
            "source_sha",
            "manifest_sha256",
            "dirty",
            "files",
            "canonical_git_objects",
            "canonical_git_tree_sha256",
            "hash_meaning",
        ],
    )?;
    require(
        value["schema"] == "humanymous.delivery-source/1"
            && value["source_sha"] == expected.source_sha
            && value["dirty"] == false
            && value["canonical_git_tree_sha256"] == expected.canonical_tree_sha256
            && value["hash_meaning"] == HASH_MEANING,
        "header source snapshot identity refused",
    )?;
    let rows = value["files"]
        .as_array()
        .ok_or("header source files absent")?;
    let objects = value["canonical_git_objects"]
        .as_array()
        .ok_or("header source Git objects absent")?;
    require(
        !expected.inputs.is_empty()
            && expected.inputs.len() <= 4096
            && rows.len() == expected.inputs.len()
            && objects.len() == expected.inputs.len(),
        "header source path denominator refused",
    )?;
    let mut seen = BTreeSet::new();
    for row in rows {
        fields(row, &["path", "bytes", "sha256"])?;
        let path = row["path"].as_str().ok_or("header source path absent")?;
        let input = expected
            .inputs
            .get(path)
            .ok_or("header source noncanonical path refused")?;
        require(
            source::safe_relative(path)
                && seen.insert(path)
                && row["bytes"] == input.bytes
                && row["sha256"] == input.sha256,
            "header source working-byte commitment refused",
        )?;
    }
    seen.clear();
    for row in objects {
        fields(row, &["path", "git_object_id"])?;
        let path = row["path"].as_str().ok_or("header Git path absent")?;
        let input = expected
            .inputs
            .get(path)
            .ok_or("header Git noncanonical path refused")?;
        require(
            seen.insert(path) && row["git_object_id"] == input.git_object_id,
            "header Git object commitment refused",
        )?;
    }
    require(
        value["manifest_sha256"]
            == source::sha(
                &serde_json::to_vec(&value["files"])
                    .map_err(|_| "header source serialization refused")?,
            ),
        "header source manifest digest refused",
    )
}

fn exact(bytes: &[u8], metadata: &Value, size: &str, hash: &str) -> Result<()> {
    require(
        metadata[size].as_u64() == Some(bytes.len() as u64) && metadata[hash] == source::sha(bytes),
        "header original-byte profile mismatch",
    )
}

pub(super) fn validate(bundle: &Bundle, profile: &Profile, expected: &Expected) -> Result<()> {
    require(
        crate::lab_receipts::digest(&Value::String(expected.source_sha.clone()), 40)
            && crate::lab_receipts::digest(
                &Value::String(expected.canonical_tree_sha256.clone()),
                64,
            ),
        "header expected source identity refused",
    )?;
    for (path, input) in &expected.inputs {
        require(
            source::safe_relative(path)
                && !path
                    .split('/')
                    .any(|part| matches!(part, ".spec.local" | ".git"))
                && source::SCOPES.iter().any(|scope| {
                    path.as_str() == *scope
                        || path
                            .strip_prefix(*scope)
                            .is_some_and(|rest| rest.starts_with('/'))
                })
                && crate::lab_receipts::digest(&Value::String(input.git_object_id.clone()), 40)
                && crate::lab_receipts::digest(&Value::String(input.sha256.clone()), 64),
            "header expected canonical input refused",
        )?;
    }
    super::file_bounds(&bundle.files, &profile.limits()?)?;
    let p = &profile.value;
    exact(&bundle.files[0], &p["header"], "bytes", "sha256")?;
    require(
        p["ownerScopeText"]
            .as_str()
            .is_some_and(|text| text.as_bytes() == bundle.files[1]),
        "header owner scope original refused",
    )?;
    for (base, language) in [(2, "c"), (8, "cpp")] {
        exact(
            &bundle.files[base],
            &p["compiler"],
            "versionBytes",
            "versionSha256",
        )?;
        exact(
            &bundle.files[base + 2],
            &p["compiler"],
            "targetBytes",
            "targetSha256",
        )?;
        exact(
            &bundle.files[base + 4],
            &p["layoutOutputs"][language],
            "bytes",
            "sha256",
        )?;
        for id in [base + 1, base + 3, base + 5] {
            require(
                bundle.files[id].is_empty(),
                "header stderr original not empty",
            )?;
        }
    }
    require(
        bundle.files[14] == bundle.files[15],
        "header original source snapshots differ",
    )?;
    snapshot(&bundle.files[14], expected, profile.limits()?.caps[14])?;
    snapshot(&bundle.files[15], expected, profile.limits()?.caps[15])?;
    let context = &bundle.context;
    fields(
        context,
        &[
            "schema",
            "source_sha",
            "run_id",
            "run_attempt",
            "job_name",
            "profile_sha256",
            "declared_image",
            "image_identity_meaning",
            "os",
            "arch",
            "compiler_identity",
            "generator_version",
            "generator_sha256",
            "generator_exit_code",
            "owner_exit_code",
            "owner_success_marker",
            "rustc_identity_sha256",
            "cargo_identity_sha256",
            "config_manifest_sha256",
            "environment_profile",
            "recipe_sha256",
            "cargo_lock_sha256",
            "original_argv_sha256",
            "argv_template",
            "argv_template_meaning",
        ],
    )?;
    require(
        context["schema"] == "humanymous.header-proof-context/1"
            && context["source_sha"] == expected.source_sha
            && expected.run_id > 0
            && expected.run_attempt > 0
            && context["run_id"] == expected.run_id
            && context["run_attempt"] == expected.run_attempt
            && context["job_name"] == expected.job_name
            && context["job_name"] == "header"
            && context["profile_sha256"] == profile.sha256
            && context["declared_image"] == p["image"]
            && context["image_identity_meaning"] == "declared-workflow-digest-not-attestation"
            && context["os"] == "linux"
            && context["arch"] == "x86_64"
            && context["generator_version"] == p["header"]["generator"]
            && context["generator_exit_code"] == 0
            && context["owner_exit_code"] == 0
            && context["owner_success_marker"] == p["ownerSuccessMarker"]
            && context["environment_profile"] == "jobs1-incremental0",
        "header expected context refused",
    )?;
    for key in [
        "generator_sha256",
        "rustc_identity_sha256",
        "cargo_identity_sha256",
        "config_manifest_sha256",
    ] {
        digest(&context[key])?;
    }
    fields(&context["compiler_identity"], &["c", "cpp"])?;
    for language in ["c", "cpp"] {
        let compiler = &context["compiler_identity"][language];
        fields(
            compiler,
            &["resolved_path", "before_sha256", "after_sha256"],
        )?;
        require(
            compiler["resolved_path"] == p["compiler"]["resolvedPath"]
                && compiler["before_sha256"] == p["compiler"]["sha256"]
                && compiler["after_sha256"] == compiler["before_sha256"],
            "header compiler binary identity refused",
        )?;
    }
    fields(&context["original_argv_sha256"], &["c", "cpp"])?;
    fields(&context["argv_template"], &["c", "cpp"])?;
    require(
        context["argv_template_meaning"] == "source-bound-template-not-original-argv",
        "header argv template meaning refused",
    )?;
    for language in ["c", "cpp"] {
        digest(&context["original_argv_sha256"][language])?;
        let (kind, standard) = if language == "c" {
            ("c", "-std=c11")
        } else {
            ("c++", "-std=c++11")
        };
        require(
            context["argv_template"][language]
                == serde_json::json!([
                    "$COMPILER_INVOCATION",
                    "-x",
                    kind,
                    "-fsyntax-only",
                    "-Xclang",
                    "-fdump-record-layouts-complete",
                    standard,
                    "$HEADER_FILE_ID_0"
                ]),
            "header source-bound argv template refused",
        )?;
    }
    let recipe = p["header"]["owningVerifier"]
        .as_str()
        .ok_or("header owning recipe path absent")?;
    require(
        context["recipe_sha256"]
            == expected
                .inputs
                .get(recipe)
                .ok_or("header canonical owning recipe absent")?
                .sha256
            && context["cargo_lock_sha256"]
                == expected
                    .inputs
                    .get("Cargo.lock")
                    .ok_or("header canonical Cargo lock absent")?
                    .sha256,
        "header recipe or dependency binding refused",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn canonical_profile_paths_and_schema_are_checked_before_file_access() {
        let profile = Profile::compiled().unwrap();
        assert_eq!(profile.paths().len(), 16);
        let value: Value = serde_json::from_slice(PROFILE_BYTES).unwrap();
        for path in [
            "../private",
            "/private",
            "header-compiler/c/version.txt",
            "source-after.json",
        ] {
            let mut bad = value.clone();
            bad["fileIdsInOrder"][0]["path"] = json!(path);
            assert!(Profile::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut extra = value;
        extra["private_metadata"] = json!("PRIVATE");
        assert!(Profile::parse(&serde_json::to_vec(&extra).unwrap()).is_err());
    }
    #[test]
    fn source_admission_binds_canonical_paths_original_hashes_and_denominators() {
        let mut inputs = BTreeMap::new();
        inputs.insert(
            "Cargo.lock".into(),
            Input {
                git_object_id: "b".repeat(40),
                bytes: 3,
                sha256: source::sha(b"abc"),
            },
        );
        let expected = Expected {
            source_sha: "a".repeat(40),
            run_id: 1,
            run_attempt: 1,
            job_name: "header".into(),
            canonical_tree_sha256: "c".repeat(64),
            inputs,
        };
        let rows = json!([{"path":"Cargo.lock","bytes":3,"sha256":source::sha(b"abc")}]);
        let good = json!({"schema":"humanymous.delivery-source/1","source_sha":expected.source_sha,"dirty":false,
            "files":rows,"manifest_sha256":source::sha(&serde_json::to_vec(&rows).unwrap()),
            "canonical_git_objects":[{"path":"Cargo.lock","git_object_id":"b".repeat(40)}],
            "canonical_git_tree_sha256":expected.canonical_tree_sha256,"hash_meaning":HASH_MEANING});
        assert!(snapshot(&serde_json::to_vec(&good).unwrap(), &expected, 4096).is_ok());
        for field in [
            "source_sha",
            "manifest_sha256",
            "canonical_git_tree_sha256",
            "hash_meaning",
        ] {
            let mut bad = good.clone();
            bad[field] = json!("PRIVATE");
            assert!(snapshot(&serde_json::to_vec(&bad).unwrap(), &expected, 4096).is_err());
        }
        for path in [
            "../Cargo.lock",
            ".spec.local/private",
            "/private",
            "Cargo.toml",
        ] {
            let mut bad = good.clone();
            bad["files"][0]["path"] = json!(path);
            assert!(snapshot(&serde_json::to_vec(&bad).unwrap(), &expected, 4096).is_err());
        }
        let mut dirty = good;
        dirty["dirty"] = json!(true);
        assert!(snapshot(&serde_json::to_vec(&dirty).unwrap(), &expected, 4096).is_err());
    }

    // Synthetic bytes exercise the admission boundary, not the actual R3 tools.
    fn fixture() -> (Bundle, Profile, Expected) {
        let mut profile = Profile::compiled().unwrap();
        let recipe = profile.value["header"]["owningVerifier"]
            .as_str()
            .unwrap()
            .to_string();
        let mut inputs = BTreeMap::new();
        for path in ["Cargo.lock".to_string(), recipe] {
            inputs.insert(
                path,
                Input {
                    git_object_id: "b".repeat(40),
                    bytes: 3,
                    sha256: source::sha(b"abc"),
                },
            );
        }
        let expected = Expected {
            source_sha: "a".repeat(40),
            run_id: 7,
            run_attempt: 1,
            job_name: "header".into(),
            canonical_tree_sha256: "c".repeat(64),
            inputs,
        };
        let rows: Vec<_> = expected
            .inputs
            .iter()
            .map(|(path, input)| json!({"path":path,"bytes":input.bytes,"sha256":input.sha256}))
            .collect();
        let objects: Vec<_> = expected
            .inputs
            .iter()
            .map(|(path, input)| json!({"path":path,"git_object_id":input.git_object_id}))
            .collect();
        let snapshot = json!({"schema":"humanymous.delivery-source/1","source_sha":expected.source_sha,"dirty":false,
            "files":rows,"manifest_sha256":source::sha(&serde_json::to_vec(&rows).unwrap()),"canonical_git_objects":objects,
            "canonical_git_tree_sha256":expected.canonical_tree_sha256,"hash_meaning":HASH_MEANING});
        let mut files: Vec<Vec<u8>> = (0..FILES).map(|_| Vec::new()).collect();
        files[0] = b"synthetic header".to_vec();
        files[1] = profile.value["ownerScopeText"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec();
        profile.value["header"]["bytes"] = json!(files[0].len());
        profile.value["header"]["sha256"] = json!(source::sha(&files[0]));
        for (base, language) in [(2, "c"), (8, "cpp")] {
            files[base] = b"synthetic compiler version\n".to_vec();
            files[base + 2] = b"synthetic-target\n".to_vec();
            files[base + 4] = format!("synthetic {language} layout\n").into_bytes();
            for (name, offset) in [("version", 0), ("target", 2)] {
                profile.value["compiler"][format!("{name}Bytes")] =
                    json!(files[base + offset].len());
                profile.value["compiler"][format!("{name}Sha256")] =
                    json!(source::sha(&files[base + offset]));
            }
            profile.value["layoutOutputs"][language]["bytes"] = json!(files[base + 4].len());
            profile.value["layoutOutputs"][language]["sha256"] =
                json!(source::sha(&files[base + 4]));
        }
        files[14] = serde_json::to_vec(&snapshot).unwrap();
        files[15] = files[14].clone();
        let compiler = json!({"resolved_path":profile.value["compiler"]["resolvedPath"],"before_sha256":profile.value["compiler"]["sha256"],"after_sha256":profile.value["compiler"]["sha256"]});
        let mut context = json!({"schema":"humanymous.header-proof-context/1","source_sha":expected.source_sha,
            "run_id":7,"run_attempt":1,"job_name":"header","profile_sha256":profile.sha256,
            "declared_image":profile.value["image"],"image_identity_meaning":"declared-workflow-digest-not-attestation",
            "os":"linux","arch":"x86_64","compiler_identity":{"c":compiler,"cpp":compiler},
            "generator_version":profile.value["header"]["generator"],"generator_sha256":"d".repeat(64),"generator_exit_code":0,
            "owner_exit_code":0,"owner_success_marker":profile.value["ownerSuccessMarker"],"rustc_identity_sha256":"d".repeat(64),
            "cargo_identity_sha256":"d".repeat(64),"config_manifest_sha256":"d".repeat(64),"environment_profile":"jobs1-incremental0",
            "recipe_sha256":source::sha(b"abc"),"cargo_lock_sha256":source::sha(b"abc"),"original_argv_sha256":{"c":"e".repeat(64),"cpp":"e".repeat(64)}});
        context["argv_template_meaning"] = json!("source-bound-template-not-original-argv");
        context["argv_template"] = json!({"c":["$COMPILER_INVOCATION","-x","c","-fsyntax-only","-Xclang","-fdump-record-layouts-complete","-std=c11","$HEADER_FILE_ID_0"],"cpp":["$COMPILER_INVOCATION","-x","c++","-fsyntax-only","-Xclang","-fdump-record-layouts-complete","-std=c++11","$HEADER_FILE_ID_0"]});
        (Bundle { context, files }, profile, expected)
    }
    #[test]
    fn whole_bundle_admission_rejects_unknown_context_tool_drift_and_nonempty_stderr() {
        let (bundle, profile, expected) = fixture();
        assert!(validate(&bundle, &profile, &expected).is_ok());
        for key in [
            "source_sha",
            "run_id",
            "run_attempt",
            "job_name",
            "profile_sha256",
            "declared_image",
            "owner_exit_code",
            "generator_exit_code",
            "recipe_sha256",
            "cargo_lock_sha256",
        ] {
            let mut bad = Bundle {
                context: bundle.context.clone(),
                files: bundle.files.clone(),
            };
            bad.context[key] = Value::Null;
            assert!(validate(&bad, &profile, &expected).is_err(), "{key}");
        }
        let mut unknown = Bundle {
            context: bundle.context.clone(),
            files: bundle.files.clone(),
        };
        unknown.context["private_env"] = json!("PRIVATE");
        assert!(validate(&unknown, &profile, &expected).is_err());
        for id in [3, 5, 7, 9, 11, 13] {
            let mut bad = Bundle {
                context: bundle.context.clone(),
                files: bundle.files.clone(),
            };
            bad.files[id].push(b'x');
            assert!(validate(&bad, &profile, &expected).is_err());
        }
        for language in ["c", "cpp"] {
            let mut bad = Bundle {
                context: bundle.context.clone(),
                files: bundle.files.clone(),
            };
            bad.context["compiler_identity"][language]["after_sha256"] = json!("0".repeat(64));
            assert!(validate(&bad, &profile, &expected).is_err());
        }
    }
}
