use crate::ast::Template;
use crate::error::{NatsuzoraError, Result};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::validator;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Template loader for handling include directives
pub struct TemplateLoader {
    include_root: PathBuf,
    cache: HashMap<String, Template>,
    include_stack: Vec<String>,
}

impl TemplateLoader {
    /// Create a new template loader with the given include root directory
    pub fn new(include_root: impl AsRef<Path>) -> Result<Self> {
        let include_root =
            include_root
                .as_ref()
                .canonicalize()
                .map_err(|e| NatsuzoraError::IncludeError {
                    message: format!("Invalid include root: {}", e),
                })?;

        Ok(Self {
            include_root,
            cache: HashMap::new(),
            include_stack: Vec::new(),
        })
    }

    /// Load a partial template by name
    pub fn load(&mut self, name: &str) -> Result<Template> {
        validator::validate_include_name_runtime(name)?;

        if self.include_stack.contains(&name.to_string()) {
            return Err(NatsuzoraError::IncludeError {
                message: format!("Circular include detected: {}", name),
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
        let path = self.resolve_path(name)?;
        self.validate_path_security(&path)?;

        if !path.exists() {
            return Err(NatsuzoraError::IncludeError {
                message: format!("Include file not found: {} ({})", name, path.display()),
            });
        }

        let source = fs::read_to_string(&path)?;
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    fn resolve_path(&self, name: &str) -> Result<PathBuf> {
        let mut segments: Vec<String> = name
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        if let Some(last) = segments.last_mut() {
            *last = format!("_{}", last);
        }

        let mut path = self.include_root.clone();
        for segment in &segments {
            path.push(segment);
        }
        path.set_extension("tmpl");

        Ok(path)
    }

    fn validate_path_security(&self, path: &Path) -> Result<()> {
        let expanded = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        if !expanded.starts_with(&self.include_root) {
            return Err(NatsuzoraError::IncludeError {
                message: format!("Path traversal detected: {}", path.display()),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Name validation tests are now in validator.rs

    #[test]
    fn test_validate_name_valid() {
        // Validation is delegated to validator module
        assert!(validator::validate_include_name_runtime("/components/card").is_ok());
        assert!(validator::validate_include_name_runtime("/a/b/c").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        // Validation is delegated to validator module
        assert!(validator::validate_include_name_runtime("no-slash").is_err());
        assert!(validator::validate_include_name_runtime("/with/../traversal").is_err());
        assert!(validator::validate_include_name_runtime("/with//double").is_err());
    }

    #[test]
    fn test_circular_include_detection() {
        let mut loader = TemplateLoader {
            include_root: env::current_dir().unwrap(),
            cache: HashMap::new(),
            include_stack: vec!["/a".to_string()],
        };

        // Simulating circular: /a is already in stack
        let result = loader.load("/a");
        assert!(matches!(result, Err(NatsuzoraError::IncludeError { .. })));
    }
}
