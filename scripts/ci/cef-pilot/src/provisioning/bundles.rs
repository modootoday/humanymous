//! Copy only pinned runtime inputs into a fresh case-specific private bundle.
use std::{
    fs, io,
    path::{Path, PathBuf},
};
pub(crate) fn fresh_bundle(source: &Path, output: &Path, name: &str) -> io::Result<PathBuf> {
    if !matches!(name, "cef-positive" | "cef-cancelled") {
        return Err(io::Error::other("unknown native case bundle"));
    }
    let output = output.canonicalize()?;
    let destination = output.join(name);
    // Existing reports, profiles or artifacts must never be reused as new evidence.
    fs::create_dir(&destination)?;
    for name in [
        "humanymous_cef_bootstrap.exe",
        "humanymous_cef_bootstrap.exe.manifest",
        "humanymous_cef_bootstrap.dll",
        "humanymous_cef_desktop.dll",
        "libcef.dll",
        "chrome_elf.dll",
        "icudtl.dat",
        "resources.pak",
        "chrome_100_percent.pak",
        "chrome_200_percent.pak",
        "v8_context_snapshot.bin",
        "archive.json",
    ] {
        super::copy_verified(&source.join(name), &destination.join(name))?;
    }
    for name in [
        "d3dcompiler_47.dll",
        "dxcompiler.dll",
        "dxil.dll",
        "libEGL.dll",
        "libGLESv2.dll",
        "vk_swiftshader.dll",
        "vk_swiftshader_icd.json",
        "vulkan-1.dll",
    ] {
        if source.join(name).try_exists()? {
            super::copy_verified(&source.join(name), &destination.join(name))?;
        }
    }
    let locales = source.join("locales");
    if fs::symlink_metadata(&locales)?.file_type().is_symlink() {
        return Err(io::Error::other("linked locale directory rejected"));
    }
    fs::create_dir(destination.join("locales"))?;
    let mut count = 0;
    for entry in fs::read_dir(locales)? {
        let entry = entry?;
        if !entry.file_type()?.is_file()
            || entry.file_type()?.is_symlink()
            || entry.path().extension().is_none_or(|ext| ext != "pak")
        {
            return Err(io::Error::other("unexpected locale runtime input"));
        }
        super::copy_verified(
            &entry.path(),
            &destination.join("locales").join(entry.file_name()),
        )?;
        count += 1;
    }
    if count == 0 || !destination.join("locales/en-US.pak").is_file() {
        return Err(io::Error::other("required native locale absent"));
    }
    Ok(destination)
}
