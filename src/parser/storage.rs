use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::config::AppConfig;
use crate::types::storage::*;
use crate::utils::to_snake;

/// Parse a storage YAML file into a list of TableDefs.
/// A single YAML file can define multiple models (like Seat.yaml defines Seat, SeatLayout, etc.).
pub fn parse_storage_yaml(yaml_path: &Path, config: &AppConfig) -> Result<Vec<TableDef>> {
    let contents = std::fs::read_to_string(yaml_path)
        .with_context(|| format!("reading {}", yaml_path.display()))?;
    let root: serde_yaml::Value = serde_yaml::from_str(&contents)
        .with_context(|| format!("parsing {}", yaml_path.display()))?;

    let root_map = root
        .as_mapping()
        .with_context(|| "YAML root must be a mapping")?;

    let imports = parse_imports(root_map);

    let mut tables = Vec::new();
    for (key, value) in root_map {
        let model_name = match key.as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        if model_name == "imports" {
            continue;
        }
        let model_map = match value.as_mapping() {
            Some(m) => m,
            None => continue,
        };

        let table = parse_single_model(&model_name, model_map, &imports, config)?;
        tables.push(table);
    }

    Ok(tables)
}

fn parse_imports(root: &serde_yaml::Mapping) -> HashMap<String, String> {
    let mut imports = HashMap::new();
    if let Some(serde_yaml::Value::Mapping(map)) =
        root.get(&serde_yaml::Value::String("imports".to_string()))
    {
        for (k, v) in map {
            if let (Some(type_name), Some(module_path)) = (k.as_str(), v.as_str()) {
                imports.insert(type_name.to_string(), module_path.to_string());
            }
        }
    }
    imports
}

fn parse_single_model(
    model_name: &str,
    model: &serde_yaml::Mapping,
    imports: &HashMap<String, String>,
    config: &AppConfig,
) -> Result<TableDef> {
    let table_name_sql = get_str(model, "tableName").unwrap_or_else(|| to_snake(model_name));

    let derives = get_str(model, "derives")
        .map(|s| s.split(',').map(|d| d.trim().to_string()).collect())
        .unwrap_or_else(|| {
            vec![
                "Debug".into(),
                "Clone".into(),
                "Serialize".into(),
                "Deserialize".into(),
            ]
        });

    let types = parse_types(model);
    let constraints = parse_constraints(model);
    let defaults = parse_defaults(model);
    let extra_indexes = parse_extra_indexes(model);

    let mut fields = parse_fields(model, &constraints, &defaults)?;

    for edf in &config.storage.extra_default_fields {
        let name = &edf.name;
        if fields.iter().any(|f| f.field_name == *name) {
            continue;
        }
        let snake = to_snake(name);
        let is_nullable = edf.rust_type.starts_with("Option<");
        fields.push(FieldDef {
            field_name: name.clone(),
            field_name_snake: snake,
            rust_type: edf.rust_type.clone(),
            is_nullable,
            is_primary_key: false,
            default_value: Some("CURRENT_TIMESTAMP".to_string()),
        });
    }

    let primary_key: Vec<String> = constraints
        .iter()
        .filter(|(_, v)| **v == FieldConstraint::PrimaryKey)
        .map(|(k, _)| k.clone())
        .collect();

    for f in &mut fields {
        if primary_key.contains(&f.field_name) {
            f.is_primary_key = true;
        }
    }

    let secondary_keys: Vec<Vec<String>> = constraints
        .iter()
        .filter(|(_, v)| **v == FieldConstraint::SecondaryKey)
        .map(|(k, _)| vec![k.clone()])
        .collect();

    let queries = parse_queries(model)?;

    Ok(TableDef {
        table_name_rust: model_name.to_string(),
        table_name_sql,
        fields,
        types,
        imports: imports.clone(),
        queries,
        primary_key,
        secondary_keys,
        derives,
        defaults,
        extra_indexes,
    })
}

fn parse_fields(
    model: &serde_yaml::Mapping,
    constraints: &HashMap<String, FieldConstraint>,
    defaults: &HashMap<String, String>,
) -> Result<Vec<FieldDef>> {
    let mut fields = Vec::new();
    if let Some(serde_yaml::Value::Mapping(map)) =
        model.get(&serde_yaml::Value::String("fields".to_string()))
    {
        for (k, v) in map {
            let name = k.as_str().unwrap_or_default().to_string();
            let rust_type = v.as_str().unwrap_or_default().to_string();
            let snake = to_snake(&name);
            let is_nullable = rust_type.starts_with("Option<");
            let is_pk = constraints.get(&name) == Some(&FieldConstraint::PrimaryKey);
            let default_value = defaults.get(&name).cloned();

            fields.push(FieldDef {
                field_name: name,
                field_name_snake: snake,
                rust_type,
                is_nullable,
                is_primary_key: is_pk,
                default_value,
            });
        }
    }
    Ok(fields)
}

fn parse_types(model: &serde_yaml::Mapping) -> Vec<TypeObject> {
    let mut result = Vec::new();
    if let Some(serde_yaml::Value::Mapping(map)) =
        model.get(&serde_yaml::Value::String("types".to_string()))
    {
        for (k, v) in map {
            let name = k.as_str().unwrap_or_default().to_string();
            if let Some(inner) = v.as_mapping() {
                let enum_str = get_str(inner, "enum");
                let extra_derives = get_str(inner, "derive")
                    .map(|s| s.split(',').map(|d| d.trim().to_string()).collect())
                    .unwrap_or_default();

                if let Some(variants_str) = enum_str {
                    let variants: Vec<String> = variants_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    result.push(TypeObject {
                        name,
                        variants,
                        is_enum: true,
                        derives: extra_derives,
                    });
                }
            }
        }
    }
    result
}

fn parse_constraints(model: &serde_yaml::Mapping) -> HashMap<String, FieldConstraint> {
    let mut map = HashMap::new();
    if let Some(serde_yaml::Value::Mapping(cmap)) =
        model.get(&serde_yaml::Value::String("constraints".to_string()))
    {
        for (k, v) in cmap {
            let field = k.as_str().unwrap_or_default().to_string();
            let constraint_str = v.as_str().unwrap_or_default();
            let constraint = match constraint_str {
                "PrimaryKey" => FieldConstraint::PrimaryKey,
                "SecondaryKey" | "!SecondaryKey" => FieldConstraint::SecondaryKey,
                "NotNull" => FieldConstraint::NotNull,
                _ => continue,
            };
            map.insert(field, constraint);
        }
    }
    map
}

fn parse_defaults(model: &serde_yaml::Mapping) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(serde_yaml::Value::Mapping(dmap)) =
        model.get(&serde_yaml::Value::String("default".to_string()))
    {
        for (k, v) in dmap {
            let field = k.as_str().unwrap_or_default().to_string();
            let val = yaml_value_to_string(v);
            map.insert(field, val);
        }
    }
    map
}

fn parse_extra_indexes(model: &serde_yaml::Mapping) -> Vec<IndexDef> {
    let mut indexes = Vec::new();
    if let Some(serde_yaml::Value::Sequence(seq)) =
        model.get(&serde_yaml::Value::String("extraIndexes".to_string()))
    {
        for item in seq {
            if let Some(obj) = item.as_mapping() {
                let columns = obj
                    .get(&serde_yaml::Value::String("columns".to_string()))
                    .and_then(|v| v.as_sequence())
                    .map(|seq| {
                        seq.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let unique = obj
                    .get(&serde_yaml::Value::String("unique".to_string()))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                indexes.push(IndexDef { columns, unique });
            }
        }
    }
    indexes
}

fn parse_queries(model: &serde_yaml::Mapping) -> Result<Vec<QueryDef>> {
    let mut queries = Vec::new();
    if let Some(serde_yaml::Value::Mapping(qmap)) =
        model.get(&serde_yaml::Value::String("queries".to_string()))
    {
        for (k, v) in qmap {
            let name = k.as_str().unwrap_or_default().to_string();
            let name_snake = to_snake(&name);

            if let Some(qdef) = v.as_mapping() {
                let kv_fn_str = get_str(qdef, "kvFunction").unwrap_or_default();
                let kv_function = match kv_fn_str.as_str() {
                    "findOneWithKV" | "findOne" => KvFunction::FindOne,
                    "findOnePartialWithKV" | "findOnePartial" => KvFunction::FindOnePartial,
                    "findAllWithKV" | "findAllWithOptionsKV" | "findAll" => KvFunction::FindAll,
                    "findAllPartialWithKV" | "findAllPartial" => KvFunction::FindAllPartial,
                    "updateWithKV" | "update" | "updateManyWithKV" => KvFunction::Update,
                    "updateOneWithKV" | "updateOne" => KvFunction::UpdateOne,
                    "deleteWithKV" | "delete" => KvFunction::Delete,
                    _ => KvFunction::FindOne,
                };

                let params = get_str_list(qdef, "params");

                let where_clause = parse_where_clause(qdef);

                let order_by = get_str(qdef, "orderBy").map(|s| {
                    if s.starts_with('-') {
                        OrderBy {
                            field: s[1..].to_string(),
                            direction: OrderDirection::Desc,
                        }
                    } else {
                        OrderBy {
                            field: s,
                            direction: OrderDirection::Asc,
                        }
                    }
                });

                queries.push(QueryDef {
                    name,
                    name_snake,
                    kv_function,
                    params,
                    where_clause,
                    order_by,
                });
            }
        }
    }
    Ok(queries)
}

fn parse_where_clause(qdef: &serde_yaml::Mapping) -> WhereClause {
    let where_val = match qdef.get(&serde_yaml::Value::String("where".to_string())) {
        Some(v) => v,
        None => return WhereClause::Empty,
    };

    parse_where_value(where_val)
}

fn parse_where_value(val: &serde_yaml::Value) -> WhereClause {
    match val {
        serde_yaml::Value::String(s) => WhereClause::Leaf(s.clone()),
        serde_yaml::Value::Mapping(map) => {
            if let Some(and_val) = map.get(&serde_yaml::Value::String("and".to_string())) {
                if let Some(seq) = and_val.as_sequence() {
                    let clauses: Vec<WhereClause> = seq.iter().map(parse_where_value).collect();
                    return WhereClause::And(clauses);
                }
            }
            if let Some(or_val) = map.get(&serde_yaml::Value::String("or".to_string())) {
                if let Some(seq) = or_val.as_sequence() {
                    let clauses: Vec<WhereClause> = seq.iter().map(parse_where_value).collect();
                    return WhereClause::Or(clauses);
                }
            }
            if let Some(in_val) = map.get(&serde_yaml::Value::String("in".to_string())) {
                if let Some(seq) = in_val.as_sequence() {
                    let fields: Vec<String> = seq
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    return WhereClause::In(fields);
                }
            }
            WhereClause::Empty
        }
        _ => WhereClause::Empty,
    }
}

fn get_str(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    map.get(&serde_yaml::Value::String(key.to_string()))
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn get_str_list(map: &serde_yaml::Mapping, key: &str) -> Vec<String> {
    map.get(&serde_yaml::Value::String(key.to_string()))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn yaml_value_to_string(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        _ => format!("{:?}", v),
    }
}
