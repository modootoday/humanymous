//! Shared custody-directory predicate; observations never initialize or repair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lookup {
    Found,
    Missing,
    Denied,
    Other,
}
#[derive(Clone, Copy, Debug)]
pub struct Facts {
    pub lookup: Lookup,
    pub directory: bool,
    pub symlink: bool,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub mode: Option<u32>,
    pub os_error: Option<i32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Missing,
    LookupDenied,
    LookupOther,
    Symlink,
    NotDirectory,
    Owner,
    Mode,
}
impl Refusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::Missing => "actor_mount_missing",
            Self::LookupDenied => "actor_mount_lookup_denied",
            Self::LookupOther => "actor_mount_lookup_other",
            Self::Symlink => "actor_mount_symlink",
            Self::NotDirectory => "actor_mount_not_directory",
            Self::Owner => "actor_mount_owner",
            Self::Mode => "actor_mount_mode",
        }
    }
}
impl Facts {
    pub fn refusal(self, uid: u32) -> Option<Refusal> {
        match self.lookup {
            Lookup::Missing => return Some(Refusal::Missing),
            Lookup::Denied => return Some(Refusal::LookupDenied),
            Lookup::Other => return Some(Refusal::LookupOther),
            Lookup::Found => {}
        }
        if self.symlink {
            Some(Refusal::Symlink)
        } else if !self.directory {
            Some(Refusal::NotDirectory)
        } else if self.uid != Some(uid) {
            Some(Refusal::Owner)
        } else if self.mode.map(|v| v & 0o777) != Some(0o700) {
            Some(Refusal::Mode)
        } else {
            None
        }
    }
    #[cfg(unix)]
    pub fn observe(path: &std::path::Path) -> Self {
        use std::os::unix::fs::MetadataExt;
        match std::fs::symlink_metadata(path) {
            Ok(m) => Self {
                lookup: Lookup::Found,
                directory: m.is_dir(),
                symlink: m.file_type().is_symlink(),
                uid: Some(m.uid()),
                gid: Some(m.gid()),
                mode: Some(m.mode()),
                os_error: None,
            },
            Err(e) => Self {
                lookup: match e.kind() {
                    std::io::ErrorKind::NotFound => Lookup::Missing,
                    std::io::ErrorKind::PermissionDenied => Lookup::Denied,
                    _ => Lookup::Other,
                },
                directory: false,
                symlink: false,
                uid: None,
                gid: None,
                mode: None,
                os_error: e.raw_os_error(),
            },
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_original_predicate_and_fail_closed_lookup_classification() {
        let good = Facts {
            lookup: Lookup::Found,
            directory: true,
            symlink: false,
            uid: Some(65532),
            gid: Some(65532),
            mode: Some(0o40700),
            os_error: None,
        };
        assert_eq!(good.refusal(65532), None);
        assert_eq!(good.gid, Some(65532));
        assert_eq!(good.os_error, None);
        for (facts, reason) in [
            (
                Facts {
                    lookup: Lookup::Missing,
                    ..good
                },
                Refusal::Missing,
            ),
            (
                Facts {
                    lookup: Lookup::Denied,
                    ..good
                },
                Refusal::LookupDenied,
            ),
            (
                Facts {
                    lookup: Lookup::Other,
                    ..good
                },
                Refusal::LookupOther,
            ),
            (
                Facts {
                    symlink: true,
                    ..good
                },
                Refusal::Symlink,
            ),
            (
                Facts {
                    directory: false,
                    ..good
                },
                Refusal::NotDirectory,
            ),
            (
                Facts {
                    uid: Some(0),
                    ..good
                },
                Refusal::Owner,
            ),
            (
                Facts {
                    mode: Some(0o755),
                    ..good
                },
                Refusal::Mode,
            ),
        ] {
            assert_eq!(facts.refusal(65532), Some(reason));
            assert!(reason.code().starts_with("actor_mount_"));
        }
        assert_eq!(
            Facts {
                uid: Some(65533),
                ..good
            }
            .refusal(65533),
            None
        );
    }
}
