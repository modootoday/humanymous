use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn walk(root: &Path, reader: bool) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    visit(root, reader, &mut result)?;
    result.sort();
    Ok(result)
}

fn visit(path: &Path, reader: bool, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!(
            "symbolic link in documentation tree: {}",
            path.display()
        ));
    }
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if reader
                && (name.starts_with('.')
                    || name.starts_with('_')
                    || ["tooling", "scripts", "vendor", "node_modules"].contains(&name.as_ref()))
            {
                continue;
            }
            visit(&entry.path(), reader, files)?;
        }
    } else if meta.is_file() {
        files.push(path.to_owned());
    }
    Ok(())
}

pub(crate) fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn contained(root: &Path, path: &Path) -> bool {
    path.canonicalize().is_ok_and(|p| p.starts_with(root))
}
