use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub output: OutputConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    pub generate: GenerateConfig,
    #[serde(default)]
    pub type_mapping: Vec<TypeMapping>,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub domain_type: String,
    pub diesel_schema: String,
    pub diesel_model: String,
    pub queries: String,
    #[serde(default)]
    pub sql: Vec<SqlOutputConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SqlOutputConfig {
    pub path: String,
    pub database: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct StorageConfig {
    #[serde(default)]
    pub extra_default_fields: Vec<ExtraDefaultField>,
}

#[derive(Debug, Deserialize)]
pub struct ExtraDefaultField {
    pub name: String,
    pub rust_type: String,
}

#[derive(Debug, Deserialize)]
pub struct GenerateConfig {
    pub generators: Vec<GeneratorKind>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum GeneratorKind {
    DomainType,
    DieselSchema,
    DieselModel,
    Queries,
    SQL,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TypeMapping {
    pub rust: String,
    pub sql: String,
    pub diesel: String,
}

impl AppConfig {
    pub fn load(config_path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(config_path)
            .with_context(|| format!("reading config file {}", config_path.display()))?;
        let config: AppConfig = toml::from_str(&contents)
            .with_context(|| format!("parsing config file {}", config_path.display()))?;
        Ok(config)
    }

    /// Resolve a relative output path against the git root.
    pub fn resolve_path(&self, relative: &str, git_root: &str) -> PathBuf {
        let expanded = relative.replace("$GIT_ROOT", git_root);
        PathBuf::from(expanded)
    }

    pub fn should_generate(&self, kind: &GeneratorKind) -> bool {
        self.generate.generators.contains(kind)
    }

    /// Returns (rust_type, diesel_type) pairs for use in diesel schema generation.
    pub fn diesel_type_mappings(&self) -> Vec<(String, String)> {
        self.type_mapping
            .iter()
            .map(|m| (m.rust.clone(), m.diesel.clone()))
            .collect()
    }

    /// Returns (rust_type, sql_type) pairs for use in SQL generation.
    pub fn sql_type_mappings(&self) -> Vec<(String, String)> {
        self.type_mapping
            .iter()
            .map(|m| (m.rust.clone(), m.sql.clone()))
            .collect()
    }
}
