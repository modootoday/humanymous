//! Resource cost-tier classification (SoT-10 §1).
use std::fmt;

/// Size above which any resource is heavy (2 MiB).
const HEAVY_BYTES: i64 = 2 << 20;
/// Size above which an image is media rather than light (256 KiB).
const MEDIA_IMAGE_BYTES: i64 = 256 << 10;

/// Resource cost class, ordered from cheapest to most expensive context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    /// T0: HTML/CSS/JS/WASM and RIT plumbing; always served.
    Essential,
    /// T1: small images and JSON.
    Light,
    /// T2: large images and audio.
    Media,
    /// T3: video and large downloads.
    Heavy,
    /// T4: iframe/embed fetch contexts.
    Embed,
}

impl Tier {
    /// Stable label used in the legacy `X-HM-Gate` header.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Essential => "T0-essential",
            Self::Light => "T1-light",
            Self::Media => "T2-media",
            Self::Heavy => "T3-heavy",
            Self::Embed => "T4-embed",
        }
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Assign a cost tier from the URL path, MIME type, body size and the
/// `Sec-Fetch-Dest` header (empty when absent).
///
/// Precedence follows the legacy rules exactly: an embed fetch destination
/// wins, then essential extensions/MIME substrings, then video, then any body
/// over 2 MiB, then audio or images over 256 KiB, and everything else is light.
/// The destination comparison is exact and case-sensitive.
pub fn classify(url_path: &str, mime: &str, size_bytes: i64, sec_fetch_dest: &str) -> Tier {
    if is_embed_destination(sec_fetch_dest) {
        return Tier::Embed;
    }
    let ext = go_lower(path_ext(url_path));
    let mime = go_lower(mime);
    let ext = ext.as_str();
    if is_essential_ext(ext)
        || ["html", "css", "javascript", "wasm"]
            .iter()
            .any(|needle| mime.contains(needle))
    {
        Tier::Essential
    } else if mime.starts_with("video/")
        || matches!(ext, ".mp4" | ".webm" | ".m3u8" | ".ts")
        || size_bytes > HEAVY_BYTES
    {
        // Video, or anything over 2 MiB.
        Tier::Heavy
    } else if mime.starts_with("audio/")
        || (mime.starts_with("image/") && size_bytes > MEDIA_IMAGE_BYTES)
    {
        Tier::Media
    } else {
        // Small images, JSON and unknown types are all light.
        Tier::Light
    }
}

/// Exact, case-sensitive embed fetch destinations.
pub(super) fn is_embed_destination(dest: &str) -> bool {
    matches!(dest, "iframe" | "embed" | "object" | "frame")
}

fn is_essential_ext(ext: &str) -> bool {
    matches!(
        ext,
        ".html" | ".htm" | ".css" | ".js" | ".mjs" | ".wasm" | ".map"
    )
}

/// Suffix from the final dot of the final slash-separated element, or empty.
fn path_ext(path: &str) -> &str {
    let name = path.rfind('/').map_or(path, |index| &path[index + 1..]);
    name.rfind('.').map_or("", |index| &name[index..])
}

/// Per-character simple lowercase mapping, matching the legacy runtime: unlike
/// `str::to_lowercase`, U+0130 maps to a plain `i` and final sigma is not
/// context-sensitive.
pub(super) fn go_lower(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\u{130}' => 'i',
            other => other.to_lowercase().next().unwrap_or(other),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_is_taken_from_final_path_element_only() {
        assert_eq!(path_ext("/dir.d/file"), "");
        assert_eq!(path_ext("/a/b.tar.GZ"), ".GZ");
        assert_eq!(path_ext("noslash.js"), ".js");
        assert_eq!(path_ext("/trailing."), ".");
        assert_eq!(path_ext(""), "");
    }

    #[test]
    fn precedence_and_strict_size_boundaries() {
        // Essential beats video and size.
        assert_eq!(classify("/x.js", "video/mp4", 9 << 20, ""), Tier::Essential);
        assert_eq!(classify("/x", "TEXT/HTML", 9 << 20, ""), Tier::Essential);
        assert_eq!(classify("/clip.TS", "", 0, ""), Tier::Heavy);
        assert_eq!(classify("/x", "", HEAVY_BYTES, ""), Tier::Light);
        assert_eq!(classify("/x", "", HEAVY_BYTES + 1, ""), Tier::Heavy);
        assert_eq!(
            classify("/x", "image/png", MEDIA_IMAGE_BYTES, ""),
            Tier::Light
        );
        assert_eq!(
            classify("/x", "image/png", MEDIA_IMAGE_BYTES + 1, ""),
            Tier::Media
        );
        assert_eq!(classify("/x", "audio/ogg", 1, ""), Tier::Media);
        assert_eq!(classify("/x", "application/json", 1, ""), Tier::Light);
        // Embed destination wins over everything, but only in exact case.
        assert_eq!(
            classify("/x.js", "text/javascript", 1, "frame"),
            Tier::Embed
        );
        assert_eq!(
            classify("/x.js", "text/javascript", 1, "IFRAME"),
            Tier::Essential
        );
    }

    #[test]
    fn lowercasing_matches_legacy_simple_mapping() {
        assert_eq!(go_lower("\u{130}MAGE/PNG"), "image/png");
        assert_eq!(classify("/x", "\u{130}mage/png", 1 << 20, ""), Tier::Media);
        assert_eq!(go_lower("\u{3a3}\u{3a3}"), "\u{3c3}\u{3c3}");
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(Tier::Essential.to_string(), "T0-essential");
        assert_eq!(Tier::Light.as_str(), "T1-light");
        assert_eq!(Tier::Media.as_str(), "T2-media");
        assert_eq!(Tier::Heavy.as_str(), "T3-heavy");
        assert_eq!(Tier::Embed.as_str(), "T4-embed");
    }
}
