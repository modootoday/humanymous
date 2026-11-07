use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub struct Directory(PathBuf);
impl Directory {
    pub fn new() -> io::Result<Self> {
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(io::Error::other)?;
        let name = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
        #[cfg(unix)]
        let root = std::env::temp_dir().canonicalize()?;
        // Windows canonicalize introduces a verbatim namespace prefix which
        // the installer intentionally refuses. Do not loosen product paths.
        #[cfg(windows)]
        let root = std::env::temp_dir();
        if !root.is_absolute() {
            return Err(io::Error::other("absolute native temp root required"));
        }
        let path = root.join(format!("humanymous-native-{name}"));
        fs::create_dir(&path)?;
        let directory = Self(path);
        private(&directory.0)?;
        Ok(directory)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub fn private(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(windows)]
    {
        use windows_permissions::{
            LocalBox, SecurityDescriptor,
            constants::{SeObjectType, SecurityInformation},
            wrappers,
        };
        let identity = humanymous_native_security::process::current_identity()?;
        let sd: LocalBox<SecurityDescriptor> =
            format!("D:P(A;OICI;FA;;;{})", identity.sid()).parse()?;
        wrappers::SetNamedSecurityInfo(
            path.as_os_str(),
            SeObjectType::SE_FILE_OBJECT,
            SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
            None,
            None,
            sd.dacl(),
            None,
        )?;
    }
    Ok(())
}
/// Validate the hosted child's own temp root and actual workspace write denial.
pub fn actor_environment(expected_sid: Option<&str>) -> io::Result<()> {
    #[cfg(windows)]
    if expected_sid.is_some() {
        fs::create_dir(std::env::temp_dir())?;
        let probe =
            std::env::current_dir()?.join(format!("native-write-probe-{}", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
        {
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => (),
            Ok(file) => {
                drop(file);
                fs::remove_file(probe)?;
                return Err(io::Error::other(
                    "workspace writable by hosted standard actor",
                ));
            }
            Err(error) => return Err(error),
        }
    }
    #[cfg(not(windows))]
    let _ = expected_sid;
    Ok(())
}
