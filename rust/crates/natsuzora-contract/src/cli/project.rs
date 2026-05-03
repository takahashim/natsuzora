use anyhow::{anyhow, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Resolve templates/contracts directories from a base project directory and optional overrides.
pub(super) fn resolve_dirs(
    base: Option<&Path>,
    templates_override: Option<&Path>,
    contracts_override: Option<&Path>,
) -> Result<(PathBuf, PathBuf)> {
    let templates_dir = match (templates_override, base) {
        (Some(p), _) => p.to_path_buf(),
        (None, Some(b)) => b.join("templates"),
        (None, None) => {
            return Err(anyhow!(
                "project directory or -T/--templates-dir is required"
            ))
        }
    };
    let contracts_dir = match (contracts_override, base) {
        (Some(p), _) => p.to_path_buf(),
        (None, Some(b)) => b.join("contracts"),
        (None, None) => {
            return Err(anyhow!(
                "project directory or -C/--contracts-dir is required"
            ))
        }
    };
    Ok((templates_dir, contracts_dir))
}

/// Resolve include root: explicit value, or the parent directory of the template.
pub(super) fn resolve_include_root(explicit: Option<&Path>, template: &Path) -> PathBuf {
    if let Some(root) = explicit {
        return root.to_path_buf();
    }
    template
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Recursively collect .ntzr files, excluding partials (files starting with `_`).
pub(super) fn collect_ntzr_files(
    base_dir: &Path,
    name_filter: Option<&str>,
) -> Result<Vec<(String, PathBuf)>> {
    collect_files(base_dir, base_dir, "ntzr", true, name_filter)
}

/// Recursively collect .ntzc files.
pub(super) fn collect_ntzc_files(
    base_dir: &Path,
    name_filter: Option<&str>,
) -> Result<Vec<(String, PathBuf)>> {
    collect_files(base_dir, base_dir, "ntzc", false, name_filter)
}

fn collect_files(
    base_dir: &Path,
    current_dir: &Path,
    extension: &str,
    exclude_underscored: bool,
    name_filter: Option<&str>,
) -> Result<Vec<(String, PathBuf)>> {
    let mut results = Vec::new();
    let entries = fs::read_dir(current_dir)
        .with_context(|| format!("reading directory {current_dir:?}"))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            results.extend(collect_files(
                base_dir,
                &path,
                extension,
                exclude_underscored,
                name_filter,
            )?);
        } else if path.extension().and_then(|e| e.to_str()) == Some(extension) {
            if exclude_underscored {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if file_name.starts_with('_') {
                    continue;
                }
            }

            let relative = path
                .strip_prefix(base_dir)
                .map_err(|_| anyhow!("path {path:?} is not under {base_dir:?}"))?;
            let name = relative
                .with_extension("")
                .to_str()
                .ok_or_else(|| anyhow!("non-UTF8 path: {relative:?}"))?
                .to_string();

            if let Some(filter) = name_filter {
                if name != filter {
                    continue;
                }
            }

            results.push((name, path));
        }
    }

    results.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(results)
}
