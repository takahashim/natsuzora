//! Template loader for handling include directives.

use crate::error::{NatsuzoraError, Result};
use natsuzora_ast::{IncludeLoader, LoaderError, Template};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

struct IncludePathResolver {
    include_root: PathBuf,
}

impl IncludePathResolver {
    fn new(include_root: impl AsRef<Path>) -> Result<Self> {
        let include_root =
            include_root
                .as_ref()
                .canonicalize()
                .map_err(|e| NatsuzoraError::IncludeError {
                    message: format!("Invalid include root: {e}"),
                })?;
        Ok(Self { include_root })
    }

    fn resolve_template_path(&self, name: &str) -> PathBuf {
        let mut segments: Vec<String> = name
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.to_string())
            .collect();

        if let Some(last) = segments.last_mut() {
            *last = format!("_{last}");
        }

        let mut path = self.include_root.clone();
        for segment in &segments {
            path.push(segment);
        }
        path.set_extension("ntzr");
        path
    }

    fn ensure_within_root(&self, path: &Path) -> Result<()> {
        let candidate = self.canonicalize_candidate(path)?;
        if self.within_root(&candidate) {
            return Ok(());
        }

        Err(NatsuzoraError::IncludeError {
            message: format!("Path traversal detected: {}", path.display()),
        })
    }

    fn canonicalize_candidate(&self, path: &Path) -> Result<PathBuf> {
        if path.exists() {
            return path
                .canonicalize()
                .map_err(|e| NatsuzoraError::IncludeError {
                    message: format!("Failed to resolve include path: {e}"),
                });
        }

        let (existing_parent, missing_segments) = split_existing_parent(path);
        let mut resolved =
            existing_parent
                .canonicalize()
                .map_err(|e| NatsuzoraError::IncludeError {
                    message: format!("Failed to resolve include path: {e}"),
                })?;
        for segment in missing_segments {
            resolved.push(segment);
        }
        Ok(resolved)
    }

    fn within_root(&self, path: &Path) -> bool {
        path == self.include_root || path.starts_with(&self.include_root)
    }
}

fn split_existing_parent(path: &Path) -> (PathBuf, Vec<String>) {
    let mut cursor = path.to_path_buf();
    let mut missing_segments = Vec::new();

    while !cursor.exists() {
        let Some(name) = cursor.file_name().and_then(|s| s.to_str()) else {
            break;
        };
        missing_segments.push(name.to_string());

        let Some(parent) = cursor.parent() else {
            break;
        };

        if parent == cursor {
            break;
        }
        cursor = parent.to_path_buf();
    }

    missing_segments.reverse();
    (cursor, missing_segments)
}

/// Template loader for handling include directives
pub struct TemplateLoader {
    path_resolver: IncludePathResolver,
    cache: HashMap<String, Template>,
    include_stack: Vec<String>,
}

impl TemplateLoader {
    /// Create a new template loader with the given include root directory
    pub fn new(include_root: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            path_resolver: IncludePathResolver::new(include_root)?,
            cache: HashMap::new(),
            include_stack: Vec::new(),
        })
    }

    /// Load a partial template by name
    pub fn load(&mut self, name: &str) -> Result<Template> {
        validate_include_name(name)?;

        if self.include_stack.contains(&name.to_string()) {
            return Err(NatsuzoraError::IncludeError {
                message: format!("Circular include detected: {name}"),
            });
        }

        if let Some(template) = self.cache.get(name) {
            return Ok(template.clone());
        }

        let template = self.load_and_parse(name)?;
        self.cache.insert(name.to_string(), template.clone());
        Ok(template)
    }

    /// Push an include name onto the stack for circular detection
    pub fn push_include(&mut self, name: &str) {
        self.include_stack.push(name.to_string());
    }

    /// Pop an include name from the stack
    pub fn pop_include(&mut self) {
        self.include_stack.pop();
    }

    fn load_and_parse(&self, name: &str) -> Result<Template> {
        let path = self.path_resolver.resolve_template_path(name);
        self.path_resolver.ensure_within_root(&path)?;

        if !path.is_file() {
            return Err(NatsuzoraError::IncludeError {
                message: format!("Include file not found: {} ({})", name, path.display()),
            });
        }

        let source = fs::read_to_string(&path)?;
        natsuzora_ast::parse(&source).map_err(|e| NatsuzoraError::IncludeError {
            message: format!("Failed to parse include '{name}': {e}"),
        })
    }
}

impl IncludeLoader for TemplateLoader {
    fn load(&mut self, name: &str) -> std::result::Result<Template, LoaderError> {
        TemplateLoader::load(self, name).map_err(|e| Box::new(e) as LoaderError)
    }
}

/// Validate include name at runtime
fn validate_include_name(name: &str) -> Result<()> {
    if !name.starts_with('/') {
        return Err(NatsuzoraError::IncludeError {
            message: format!("Include name must start with '/': {name}"),
        });
    }

    if name.contains("..") || name.contains("//") || name.contains('\\') || name.contains(':') {
        return Err(NatsuzoraError::IncludeError {
            message: format!("Invalid include name (path traversal): {name}"),
        });
    }

    for segment in name.split('/').filter(|s| !s.is_empty()) {
        if !is_valid_segment(segment) {
            return Err(NatsuzoraError::IncludeError {
                message: format!("Invalid include segment '{segment}' in '{name}'"),
            });
        }
    }

    Ok(())
}

fn is_valid_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_include_name("/components/card").is_ok());
        assert!(validate_include_name("/a/b/c").is_ok());
        assert!(validate_include_name("/shared/layout/header").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(validate_include_name("no-slash").is_err());
        assert!(validate_include_name("/with/../traversal").is_err());
        assert!(validate_include_name("/with//double").is_err());
        assert!(validate_include_name("/with-dash").is_err());
    }

    #[test]
    fn test_circular_include_detection() {
        let mut loader = TemplateLoader {
            path_resolver: IncludePathResolver {
                include_root: env::current_dir().unwrap(),
            },
            cache: HashMap::new(),
            include_stack: vec!["/a".to_string()],
        };

        let result = loader.load("/a");
        assert!(matches!(result, Err(NatsuzoraError::IncludeError { .. })));
    }
}
