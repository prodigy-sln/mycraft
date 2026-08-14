//! A declared value exactly as some source format spelled it.
//!
//! The model checks declarations, and a check that names the field at fault has
//! to survive a value of the wrong kind — `size = ["9", 1]` must be refused
//! *naming the element and the field*, which a parser that failed on the type
//! could not do because it would have thrown the name away first.
//!
//! This crate cannot name a serialization format (its resolved dependency graph
//! is asserted to reach none), so the untyped value is its own, and whoever
//! reads a file converts into it.

/// One value as a declaration spelled it, before anything has been checked.
#[derive(Debug, Clone)]
pub enum DeclaredValue {
    Text(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    List(Vec<DeclaredValue>),
    /// A value this model has no use for, carrying the name its source format
    /// calls it by — a nested table, a datetime, anything a later format adds.
    ///
    /// Present so that converting into this type is total: a reader that had to
    /// drop such a value could not report what it found.
    Opaque(String),
}

impl DeclaredValue {
    /// What this value is, in the words a refusal quotes back to whoever wrote
    /// the declaration.
    pub fn kind(&self) -> &str {
        match self {
            Self::Text(_) => "a string",
            Self::Integer(_) => "an integer",
            Self::Decimal(_) => "a decimal number",
            Self::Boolean(_) => "true or false",
            Self::List(_) => "a list",
            Self::Opaque(kind) => kind,
        }
    }
}
