//! Buffered original-byte transport core. Profile admission and filesystem IO
//! remain separate; this module alone cannot export or qualify a header proof.
use crate::{Result, require, source};
use serde_json::{Value, json};
mod actions_log;
mod admission;
mod codec;
mod frontend;
mod private_io;
mod source_index;
pub(crate) use frontend::{observe, parent, plan, prepare, receive};

const PREFIX: &str = "HUMANYMOUS_HEADER_PROOF_V1 ";
const FILES: usize = 16;

struct Limits {
    caps: [usize; FILES],
    total: usize,
    chunk: usize,
    line: usize,
    log: usize,
}
impl Limits {
    fn profile(profile: &Value) -> Result<Self> {
        let limit = |name: &str| -> Result<usize> {
            profile["limits"][name]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .filter(|n| *n > 0 && *n <= 16 * 1024 * 1024)
                .ok_or("proof profile limit refused".into())
        };
        let mut caps = [limit("otherFileBytesEach")?; FILES];
        caps[0] = limit("headerBytes")?;
        caps[14] = limit("sourceSnapshotBytesEach")?;
        caps[15] = caps[14];
        Ok(Self {
            caps,
            total: limit("totalDecodedBytes")?,
            chunk: limit("decodedChunkBytes")?,
            line: limit("protocolLineBytes")?,
            log: limit("retainedLogBytes")?,
        })
    }
}

struct Bundle {
    context: Value,
    files: Vec<Vec<u8>>,
}

fn fields(value: &Value, keys: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or("proof object refused")?;
    require(
        object.len() == keys.len() && object.keys().all(|key| keys.contains(&key.as_str())),
        "proof fields refused",
    )
}
fn number(value: &Value) -> Result<usize> {
    value
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or("proof integer refused".into())
}
fn file_bounds(files: &[Vec<u8>], limits: &Limits) -> Result<()> {
    require(files.len() == FILES, "proof file count refused")?;
    let mut total = 0usize;
    for (index, bytes) in files.iter().enumerate() {
        total = total
            .checked_add(bytes.len())
            .ok_or("proof total overflow")?;
        require(
            bytes.len() <= limits.caps[index] && total <= limits.total,
            "proof decoded bound refused",
        )?;
    }
    Ok(())
}
fn line(value: &Value, limits: &Limits) -> Result<String> {
    let text = format!(
        "{PREFIX}{}\n",
        serde_json::to_string(value).map_err(|_| "proof encoding failed")?
    );
    require(text.len() <= limits.line, "proof line bound refused")?;
    Ok(text)
}

fn encode(
    bundle: &Bundle,
    limits: &Limits,
    admit: impl Fn(&Bundle) -> Result<()>,
) -> Result<String> {
    file_bounds(&bundle.files, limits)?;
    // The complete admission callback runs before even the begin frame exists.
    admit(bundle)?;
    let entries: Vec<_> = bundle
        .files
        .iter()
        .enumerate()
        .map(|(id, bytes)| json!({"id":id,"bytes":bytes.len(),"sha256":source::sha(bytes)}))
        .collect();
    let manifest = json!({"context":bundle.context,"files":entries});
    let mut out = line(&json!({"type":"begin","manifest":manifest}), limits)?;
    for (id, bytes) in bundle.files.iter().enumerate() {
        let total = bytes.len().div_ceil(limits.chunk).max(1);
        for seq in 0..total {
            let start = seq * limits.chunk;
            let end = start.saturating_add(limits.chunk).min(bytes.len());
            out.push_str(&line(
                &json!({"type":"chunk","id":id,"seq":seq,"total":total,
                "base64":codec::encode(&bytes[start..end])}),
                limits,
            )?);
            require(out.len() <= limits.log, "proof log bound refused")?;
        }
    }
    out.push_str(&line(&json!({"type":"end","manifest_sha256":source::sha(&serde_json::to_vec(&manifest).map_err(|_| "proof manifest encoding failed")?)}), limits)?);
    require(out.len() <= limits.log, "proof log bound refused")?;
    Ok(out)
}

fn decode(text: &str, limits: &Limits, admit: impl Fn(&Bundle) -> Result<()>) -> Result<Bundle> {
    require(
        text.ends_with('\n'),
        "proof unterminated retained log refused",
    )?;
    require(text.len() <= limits.log, "proof retained log bound refused")?;
    let mut manifest: Option<Value> = None;
    let mut sizes = Vec::new();
    let mut hashes = Vec::new();
    let mut files: Vec<Vec<u8>> = (0..FILES).map(|_| Vec::new()).collect();
    let mut file = 0usize;
    let mut seq = 0usize;
    let mut ended = false;
    for line in text.lines() {
        let Some(raw) = line.strip_prefix(PREFIX) else {
            continue;
        };
        require(!ended, "proof trailing protocol refused")?;
        require(
            line.len() < limits.line,
            "proof protocol line bound refused",
        )?;
        let frame =
            humanymous_platform::testing::json::decode_strict_json(raw.as_bytes(), limits.line)
                .map_err(|_| "proof JSON refused")?;
        match frame["type"].as_str() {
            Some("begin") => {
                fields(&frame, &["type", "manifest"])?;
                require(manifest.is_none(), "proof duplicate envelope refused")?;
                let m = &frame["manifest"];
                fields(m, &["context", "files"])?;
                let rows = m["files"]
                    .as_array()
                    .ok_or("proof manifest files refused")?;
                require(rows.len() == FILES, "proof manifest file count refused")?;
                let mut total = 0usize;
                for (index, row) in rows.iter().enumerate() {
                    fields(row, &["id", "bytes", "sha256"])?;
                    require(number(&row["id"])? == index, "proof file ID refused")?;
                    let size = number(&row["bytes"])?;
                    total = total
                        .checked_add(size)
                        .ok_or("proof manifest total overflow")?;
                    require(
                        size <= limits.caps[index] && total <= limits.total,
                        "proof manifest bound refused",
                    )?;
                    require(
                        crate::lab_receipts::digest(&row["sha256"], 64),
                        "proof manifest digest refused",
                    )?;
                    sizes.push(size);
                    hashes.push(
                        row["sha256"]
                            .as_str()
                            .ok_or("proof digest refused")?
                            .to_string(),
                    );
                }
                manifest = Some(m.clone());
            }
            Some("chunk") => {
                fields(&frame, &["type", "id", "seq", "total", "base64"])?;
                require(
                    manifest.is_some() && file < FILES,
                    "proof unexpected chunk refused",
                )?;
                require(
                    number(&frame["id"])? == file && number(&frame["seq"])? == seq,
                    "proof chunk order refused",
                )?;
                let total = sizes[file].div_ceil(limits.chunk).max(1);
                require(
                    number(&frame["total"])? == total,
                    "proof chunk total refused",
                )?;
                let bytes = codec::decode(
                    frame["base64"].as_str().ok_or("proof chunk data refused")?,
                    limits.chunk,
                )?;
                let expected = sizes[file]
                    .saturating_sub(files[file].len())
                    .min(limits.chunk);
                require(bytes.len() == expected, "proof chunk length refused")?;
                files[file].extend(bytes);
                seq += 1;
                if seq == total {
                    require(
                        source::sha(&files[file]) == hashes[file],
                        "proof original-byte digest refused",
                    )?;
                    file += 1;
                    seq = 0;
                }
            }
            Some("end") => {
                fields(&frame, &["type", "manifest_sha256"])?;
                let m = manifest
                    .as_ref()
                    .ok_or("proof completion without manifest")?;
                require(file == FILES && seq == 0, "proof incomplete files refused")?;
                require(
                    frame["manifest_sha256"]
                        == source::sha(
                            &serde_json::to_vec(m).map_err(|_| "proof manifest encoding failed")?,
                        ),
                    "proof completion digest refused",
                )?;
                ended = true;
            }
            _ => return Err("proof frame type refused".into()),
        }
    }
    require(ended, "proof missing completion refused")?;
    let manifest = manifest.ok_or("proof manifest absent")?;
    let bundle = Bundle {
        context: manifest["context"].clone(),
        files,
    };
    file_bounds(&bundle.files, limits)?;
    admit(&bundle)?;
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Bundle, Limits) {
        let profile: Value = serde_json::from_str(include_str!(
            "../../../../adapters/native/header-proof-profile.json"
        ))
        .unwrap();
        let limits = Limits::profile(&profile).unwrap();
        let files = (0..FILES)
            .map(|id| {
                if id.is_multiple_of(3) {
                    vec![]
                } else {
                    vec![id as u8, 0, 255, b'\n', b'\r']
                }
            })
            .collect();
        (
            Bundle {
                context: json!({"synthetic_transport_fixture":true}),
                files,
            },
            limits,
        )
    }
    #[test]
    fn sixteen_original_files_including_empty_and_binary_bytes_round_trip() {
        let (bundle, limits) = fixture();
        let text = encode(&bundle, &limits, |_| Ok(())).unwrap();
        let received = decode(&text, &limits, |_| Ok(())).unwrap();
        assert_eq!(received.files, bundle.files);
        assert_eq!(received.context, bundle.context);
        assert_eq!(text.lines().count(), FILES + 2);
        assert!(encode(&bundle, &limits, |_| Err("profile refused".into())).is_err());
        assert!(decode(&text, &limits, |_| Err("profile refused".into())).is_err());
    }
    #[test]
    fn duplicate_gapped_reordered_truncated_and_extra_frames_are_refused() {
        let (bundle, limits) = fixture();
        let text = encode(&bundle, &limits, |_| Ok(())).unwrap();
        assert!(decode(text.trim_end_matches('\n'), &limits, |_| Ok(())).is_err());
        let lines: Vec<_> = text.lines().collect();
        let check = |parts: Vec<&str>| {
            assert!(decode(&(parts.join("\n") + "\n"), &limits, |_| Ok(())).is_err())
        };
        for index in 0..lines.len() {
            let mut missing = lines.clone();
            missing.remove(index);
            check(missing);
            let mut duplicate = lines.clone();
            duplicate.insert(index, lines[index]);
            check(duplicate);
        }
        let mut swapped = lines.clone();
        swapped.swap(2, 3);
        check(swapped);
        check([lines.clone(), lines.clone()].concat());
        for (old, new) in [
            ("\"seq\":0", "\"seq\":1"),
            ("\"total\":1", "\"total\":2"),
            ("\"id\":0", "\"id\":16"),
            ("\"type\":\"chunk\"", "\"type\":\"unknown\""),
            (
                "\"type\":\"begin\"",
                "\"type\":\"begin\",\"type\":\"begin\"",
            ),
        ] {
            assert!(decode(&text.replacen(old, new, 1), &limits, |_| Ok(())).is_err());
        }
        let mut oversized = Bundle {
            context: bundle.context,
            files: bundle.files,
        };
        oversized.files[1] = vec![0; limits.caps[1] + 1];
        assert!(encode(&oversized, &limits, |_| Ok(())).is_err());
    }

    #[test]
    fn multichunk_lengths_hashes_and_closed_file_metadata_are_checked() {
        let (mut bundle, limits) = fixture();
        bundle.files[1] = vec![0x91; limits.chunk + 1];
        let text = encode(&bundle, &limits, |_| Ok(())).unwrap();
        assert_eq!(
            decode(&text, &limits, |_| Ok(())).unwrap().files,
            bundle.files
        );
        let wrong_hash = text.replacen(&source::sha(&[]), &"0".repeat(64), 1);
        assert_eq!(
            decode(&wrong_hash, &limits, |_| Ok(())).err().as_deref(),
            Some("proof original-byte digest refused")
        );
        let path = text.replacen("\"id\":0", "\"id\":0,\"path\":\"../private\"", 1);
        assert_eq!(
            decode(&path, &limits, |_| Ok(())).err().as_deref(),
            Some("proof fields refused")
        );
        let bad_padding = text.replacen("kZGR", "kZG=", 1);
        assert!(decode(&bad_padding, &limits, |_| Ok(())).is_err());
        let mut too_long = text;
        too_long.push_str(&"x".repeat(limits.log));
        too_long.push('\n');
        assert_eq!(
            decode(&too_long, &limits, |_| Ok(())).err().as_deref(),
            Some("proof retained log bound refused")
        );
    }
}
