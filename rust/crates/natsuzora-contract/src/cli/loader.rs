use anyhow::{anyhow, Context, Result};
use natsuzora::ast::{IncludeLoader, LoaderError, Template};
use std::{collections::HashMap, fs, path::PathBuf};

pub(super) struct FileIncludeLoader {
    root: PathBuf,
    cache: HashMap<String, Template>,
}

impl FileIncludeLoader {
    pub(super) fn new(root: PathBuf) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("resolving include root {root:?}"))?;
        if !root.is_dir() {
            return Err(anyhow!("include root is not a directory: {root:?}"));
        }

        Ok(Self {
            root,
            cache: HashMap::new(),
        })
    }

    fn resolve_path(&self, name: &str) -> Result<PathBuf> {
        if !name.starts_with('/') {
            return Err(anyhow!("include name '{name}' must start with '/'"));
        }
        let segments: Vec<&str> = name
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if segments.is_empty() {
            return Err(anyhow!("include name '{name}' is empty"));
        }
        for seg in &segments {
            if !is_valid_segment(seg) {
                return Err(anyhow!("invalid include segment '{seg}' in '{name}'"));
            }
        }
        let mut path = self.root.clone();
        if segments.len() > 1 {
            for seg in &segments[..segments.len() - 1] {
                path.push(seg);
            }
        }
        path.push(format!("_{}.ntzr", segments.last().unwrap()));
        Ok(path)
    }

    fn resolve_secure_path(&self, name: &str) -> Result<PathBuf> {
        let path = self.resolve_path(name)?;
        let candidate = path
            .canonicalize()
            .with_context(|| format!("resolving include {path:?}"))?;
        if !candidate.starts_with(&self.root) {
            return Err(anyhow!(
                "include path escapes include root: {}",
                path.display()
            ));
        }
        Ok(candidate)
    }
}

fn is_valid_segment(seg: &str) -> bool {
    let mut chars = seg.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl IncludeLoader for FileIncludeLoader {
    fn load(&mut self, name: &str) -> Result<Template, LoaderError> {
        if let Some(template) = self.cache.get(name) {
            return Ok(template.clone());
        }
        let path = self.resolve_secure_path(name)?;
        let source =
            fs::read_to_string(&path).with_context(|| format!("reading include {path:?}"))?;
        let template = natsuzora::ast::parse(&source)?;
        self.cache.insert(name.to_string(), template.clone());
        Ok(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_include_outside_root() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("_secret.ntzr");
        fs::write(&outside_file, "{[ secret ]}").unwrap();
        symlink(&outside_file, root.path().join("_secret.ntzr")).unwrap();

        let mut loader = FileIncludeLoader::new(root.path().to_path_buf()).unwrap();
        let err = loader.load("/secret").unwrap_err();
        assert!(err.to_string().contains("escapes include root"));
    }

    #[test]
    fn loads_regular_include_inside_root() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("_card.ntzr"), "{[ title ]}").unwrap();

        let mut loader = FileIncludeLoader::new(root.path().to_path_buf()).unwrap();
        assert!(loader.load("/card").is_ok());
    }
}
