mod config;
mod generator;
mod parser;
mod types;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use config::{AppConfig, GeneratorKind};
use utils::FileState;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(
    name = "namma-dsl-rs",
    version,
    about = "Generate Rust+Diesel storage code from YAML specs"
)]
struct Cli {
    /// Regenerate all files regardless of git state
    #[arg(long)]
    all: bool,

    /// Only process a specific spec directory
    #[arg(long)]
    path: Option<String>,

    /// Skip running cargo fmt after generation
    #[arg(long)]
    skip_fmt: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("namma-dsl-rs v{}", VERSION);

    let git_root = utils::find_git_root().context("finding git root")?;
    std::env::set_var("GIT_ROOT", &git_root);
    println!("\x1b[32mRoot dir: {}\x1b[0m", git_root);

    let root_path = PathBuf::from(&git_root);

    if let Some(ref specific_path) = cli.path {
        let spec_dir = root_path.join(specific_path);
        process_spec_dir(&spec_dir, &git_root, cli.all)?;
    } else {
        walk_and_process(&root_path, &git_root, cli.all)?;
    }

    if !cli.skip_fmt {
        println!("Running cargo fmt...");
        let status = std::process::Command::new("cargo")
            .arg("fmt")
            .current_dir(&root_path)
            .status();
        match status {
            Ok(s) if s.success() => println!("cargo fmt completed"),
            Ok(s) => eprintln!("cargo fmt exited with: {}", s),
            Err(e) => eprintln!("cargo fmt failed to run: {}", e),
        }
    }

    println!("\x1b[32mDone.\x1b[0m");
    Ok(())
}

fn walk_and_process(root: &Path, git_root: &str, gen_all: bool) -> Result<()> {
    for entry in WalkDir::new(root).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !name.starts_with('.') && name != "target" && name != "node_modules"
    }) {
        let entry = entry?;
        if entry.file_type().is_dir() && entry.file_name() == "spec" {
            let config_path = entry.path().join("dsl-config.toml");
            if config_path.exists() {
                process_spec_dir(entry.path(), git_root, gen_all)?;
            }
        }
    }
    Ok(())
}

fn process_spec_dir(spec_dir: &Path, git_root: &str, gen_all: bool) -> Result<()> {
    let config_path = spec_dir.join("dsl-config.toml");
    if !config_path.exists() {
        println!(
            "\x1b[33mSkipping {} (no dsl-config.toml)\x1b[0m",
            spec_dir.display()
        );
        return Ok(());
    }

    println!("\x1b[32mProcessing spec dir: {}\x1b[0m", spec_dir.display());

    let config = AppConfig::load(&config_path)?;
    let storage_dir = spec_dir.join("Storage");

    if !storage_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(&storage_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }

        let file_state = utils::get_file_state(&path);
        let should_run =
            gen_all || file_state == FileState::New || file_state == FileState::Changed;

        if !should_run {
            println!("  Skipping {} (unchanged)", path.display());
            continue;
        }

        println!("  Generating from {}...", path.display());
        process_yaml_file(&path, &config, git_root)?;
    }

    Ok(())
}

fn process_yaml_file(yaml_path: &Path, config: &AppConfig, git_root: &str) -> Result<()> {
    let tables = parser::storage::parse_storage_yaml(yaml_path, config)?;
    let diesel_type_mappings = config.diesel_type_mappings();
    let sql_type_mappings = config.sql_type_mappings();

    for table in &tables {
        if config.should_generate(&GeneratorKind::DomainType) {
            let code = generator::domain_type::generate_domain_type(table);
            let dir = config.resolve_path(&config.output.domain_type, git_root);
            let filename = format!("{}.rs", utils::to_snake(&table.table_name_rust));
            utils::write_to_file(&dir, &filename, &code)?;
            println!("    -> {}/{}", dir.display(), filename);
        }

        if config.should_generate(&GeneratorKind::DieselSchema) {
            let code =
                generator::diesel_schema::generate_diesel_schema(table, &diesel_type_mappings);
            let dir = config.resolve_path(&config.output.diesel_schema, git_root);
            let filename = format!("{}.rs", utils::to_snake(&table.table_name_rust));
            utils::write_to_file(&dir, &filename, &code)?;
            println!("    -> {}/{}", dir.display(), filename);
        }

        if config.should_generate(&GeneratorKind::DieselModel) {
            let code = generator::diesel_model::generate_diesel_model(table);
            let dir = config.resolve_path(&config.output.diesel_model, git_root);
            let filename = format!("{}.rs", utils::to_snake(&table.table_name_rust));
            utils::write_to_file(&dir, &filename, &code)?;
            println!("    -> {}/{}", dir.display(), filename);
        }

        if config.should_generate(&GeneratorKind::Queries) {
            let code = generator::queries::generate_queries(table);
            let dir = config.resolve_path(&config.output.queries, git_root);
            let filename = format!("{}.rs", utils::to_snake(&table.table_name_rust));
            utils::write_to_file(&dir, &filename, &code)?;
            println!("    -> {}/{}", dir.display(), filename);
        }

        if config.should_generate(&GeneratorKind::SQL) {
            for sql_cfg in &config.output.sql {
                let dir = config.resolve_path(&sql_cfg.path, git_root);
                let filename = format!("{}.sql", table.table_name_sql);
                let existing_path = dir.join(&filename);
                let existing = if existing_path.exists() {
                    Some(existing_path.as_path())
                } else {
                    None
                };
                let code = generator::sql::generate_sql(
                    table,
                    &sql_cfg.database,
                    &sql_type_mappings,
                    existing,
                );
                utils::write_to_file(&dir, &filename, &code)?;
                println!("    -> {}/{}", dir.display(), filename);
            }
        }
    }

    Ok(())
}
