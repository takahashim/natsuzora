use anyhow::{anyhow, Context, Result};
use natsuzora_ast::{IncludeLoader, LoaderError, Template};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
};

pub(super) struct FileIncludeLoader {
    root: PathBuf,
    cache: HashMap<String, Template>,
}

impl FileIncludeLoader {
    pub(super) fn new(root: PathBuf) -> Self {
        Self {
            root,
            cache: HashMap::new(),
        }
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
        let path = self.resolve_path(name)?;
        let source =
            fs::read_to_string(&path).with_context(|| format!("reading include {path:?}"))?;
        let template = natsuzora_ast::parse(&source)?;
        self.cache.insert(name.to_string(), template.clone());
        Ok(template)
    }
}
