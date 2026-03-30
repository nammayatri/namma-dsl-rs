use std::fmt::Write;
use std::path::Path;

use crate::types::storage::TableDef;
use crate::utils;

/// Generate SQL migration statements for a single table.
/// If an existing migration file exists, appends incremental changes.
pub fn generate_sql(
    table: &TableDef,
    database: &str,
    sql_type_mappings: &[(String, String)],
    existing_migration_path: Option<&Path>,
) -> String {
    let enum_names: Vec<String> = table.types.iter().map(|t| t.name.clone()).collect();

    let existing_content = existing_migration_path.and_then(|p| std::fs::read_to_string(p).ok());

    if let Some(ref existing) = existing_content {
        generate_incremental_sql(table, database, &enum_names, sql_type_mappings, existing)
    } else {
        generate_fresh_sql(table, database, &enum_names, sql_type_mappings)
    }
}

fn generate_fresh_sql(
    table: &TableDef,
    database: &str,
    enum_names: &[String],
    type_mappings: &[(String, String)],
) -> String {
    let mut out = String::new();
    let full_table = format!("{}.{}", database, table.table_name_sql);

    writeln!(out, "CREATE TABLE {} ();", full_table).unwrap();
    writeln!(out).unwrap();

    for f in &table.fields {
        let sql_type = utils::rust_type_to_sql_type(&f.rust_type, enum_names, type_mappings);
        let not_null = if !f.is_nullable { " NOT NULL" } else { "" };
        let default = f
            .default_value
            .as_ref()
            .map(|d| format!(" default {}", d))
            .unwrap_or_default();

        writeln!(
            out,
            "ALTER TABLE {} ADD COLUMN {}{}{}{};",
            full_table,
            f.field_name_snake,
            sql_type_padded(&sql_type),
            not_null,
            default
        )
        .unwrap();
    }

    // Primary key
    if !table.primary_key.is_empty() {
        let pk_cols: Vec<String> = table
            .primary_key
            .iter()
            .map(|p| utils::to_snake(p))
            .collect();
        writeln!(
            out,
            "ALTER TABLE {} ADD PRIMARY KEY ( {});",
            full_table,
            pk_cols.join(", ")
        )
        .unwrap();
    }

    // Secondary key indexes
    for sk in &table.secondary_keys {
        let cols: Vec<String> = sk.iter().map(|c| utils::to_snake(c)).collect();
        let idx_name = format!("idx_{}_{}", table.table_name_sql, cols.join("_"));
        writeln!(
            out,
            "CREATE INDEX {} ON {} ({});",
            idx_name,
            full_table,
            cols.join(", ")
        )
        .unwrap();
    }

    // Extra indexes
    for idx in &table.extra_indexes {
        let cols: Vec<String> = idx.columns.iter().map(|c| utils::to_snake(c)).collect();
        let idx_name = format!("idx_{}_{}", table.table_name_sql, cols.join("_"));
        if idx.unique {
            writeln!(
                out,
                "ALTER TABLE {} ADD CONSTRAINT {} UNIQUE ({});",
                full_table,
                idx_name,
                cols.join(", ")
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "CREATE INDEX {} ON {} ({});",
                idx_name,
                full_table,
                cols.join(", ")
            )
            .unwrap();
        }
    }

    out
}

fn generate_incremental_sql(
    table: &TableDef,
    database: &str,
    enum_names: &[String],
    type_mappings: &[(String, String)],
    existing: &str,
) -> String {
    let full_table = format!("{}.{}", database, table.table_name_sql);
    let existing_columns = parse_existing_columns(existing);

    let mut additions = String::new();

    for f in &table.fields {
        if existing_columns.contains(&f.field_name_snake) {
            continue;
        }
        let sql_type = utils::rust_type_to_sql_type(&f.rust_type, enum_names, type_mappings);
        let not_null = if !f.is_nullable { " NOT NULL" } else { "" };
        let default = f
            .default_value
            .as_ref()
            .map(|d| format!(" default {}", d))
            .unwrap_or_default();

        writeln!(
            additions,
            "ALTER TABLE {} ADD COLUMN {}{}{}{};",
            full_table,
            f.field_name_snake,
            sql_type_padded(&sql_type),
            not_null,
            default
        )
        .unwrap();
    }

    if additions.is_empty() {
        return existing.to_string();
    }

    let mut out = existing.trim_end().to_string();
    writeln!(out).unwrap();
    writeln!(out).unwrap();
    writeln!(out, "------- SQL updates -------").unwrap();
    writeln!(out).unwrap();
    write!(out, "{}", additions).unwrap();
    out
}

fn parse_existing_columns(sql: &str) -> Vec<String> {
    let mut cols = Vec::new();
    for line in sql.lines() {
        let trimmed = line.trim();
        if trimmed.to_uppercase().contains("ADD COLUMN ") {
            let actual_rest = &trimmed[trimmed.to_uppercase().find("ADD COLUMN ").unwrap() + 11..];
            if let Some(col_name) = actual_rest.split_whitespace().next() {
                cols.push(col_name.to_string());
            }
        }
    }
    cols
}

fn sql_type_padded(sql_type: &str) -> String {
    format!(" {}", sql_type)
}
