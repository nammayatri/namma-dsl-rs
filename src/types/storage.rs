use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TableDef {
    pub table_name_rust: String,
    pub table_name_sql: String,
    pub fields: Vec<FieldDef>,
    pub types: Vec<TypeObject>,
    pub imports: HashMap<String, String>,
    pub queries: Vec<QueryDef>,
    pub primary_key: Vec<String>,
    pub secondary_keys: Vec<Vec<String>>,
    pub derives: Vec<String>,
    pub defaults: HashMap<String, String>,
    pub extra_indexes: Vec<IndexDef>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub field_name: String,
    pub field_name_snake: String,
    pub rust_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct QueryDef {
    pub name: String,
    pub name_snake: String,
    pub kv_function: KvFunction,
    pub params: Vec<String>,
    pub where_clause: WhereClause,
    pub order_by: Option<OrderBy>,
}

#[derive(Debug, Clone)]
pub enum WhereClause {
    Empty,
    Leaf(String),
    And(Vec<WhereClause>),
    Or(Vec<WhereClause>),
    In(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum KvFunction {
    FindOne,
    FindAll,
    Update,
    UpdateOne,
    Delete,
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub field: String,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldConstraint {
    PrimaryKey,
    SecondaryKey,
    NotNull,
}

#[derive(Debug, Clone)]
pub struct TypeObject {
    pub name: String,
    pub variants: Vec<String>,
    pub is_enum: bool,
    pub derives: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IndexDef {
    pub columns: Vec<String>,
    pub unique: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Import {
    pub type_name: String,
    pub module_path: String,
}
