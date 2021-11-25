use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Mapping {
    lowest: TypeMap,
    highest: TypeMap,
    conditional: Option<TypeMap>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeMap {
    Null,
    Boolean,
    SignedInteger,
    UnsignedInteger,
    Double,
    String,
    IPv4,
    IPv6,
    Date,
    Array(BTreeMap<usize, TypeMap>),
    Object(BTreeMap<String, TypeMap>),
}

impl Default for TypeMap {
    fn default() -> Self {
        TypeMap::Object(BTreeMap::new())
    }
}

impl TypeMap {
    pub fn return_type(value: &JsonValue) -> TypeMap {
        match value {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(_) => Self::Boolean,
            JsonValue::Number(n) => Self::map_number(n),
            JsonValue::String(s) => Self::map_string(s.as_str()),
            JsonValue::Array(_) => Self::Array(BTreeMap::new()),
            JsonValue::Object(_) => Self::Object(BTreeMap::new()),
        }
    }

    pub fn map_number(value: &serde_json::Number) -> Self {
        if value.is_u64() {
            Self::UnsignedInteger
        } else if value.is_i64() {
            Self::SignedInteger
        } else {
            Self::Double
        }
    }

    pub fn cmp_number(value: &serde_json::Number, right: &mut Self) {
        let left = Self::map_number(value);
        *right = match (&left, &right) {
            (Self::SignedInteger, Self::UnsignedInteger) => left,
            (Self::Double, Self::UnsignedInteger | Self::SignedInteger) => left,
            _ => return,
        }
    }

    pub fn map_string(value: &str) -> Self {
        if value.to_ascii_lowercase() == "true" || value.to_ascii_lowercase() == "false" {
            Self::Boolean
        } else if chrono::DateTime::parse_from_rfc3339(value).is_ok()
            || chrono::DateTime::parse_from_rfc2822(value).is_ok()
            || chrono::DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.6fZ").is_ok()
            || chrono::DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.3fZ").is_ok()
        {
            Self::Date
        }
        // else if value.starts_with("0x") {
        //     // Possible number?
        // }
        else {
            Self::String
        }
    }

    pub fn cmp_string(value: &str, right: &mut Self) {
        // Fail early if right is a string
        if let Self::String = right {
            return;
        }
        // Complex string parsing
        let left = Self::map_string(value);
        *right = match (&left, &right) {
            (Self::String, Self::Boolean) => left,
            // (TypeMap::String, TypeMap::UnsignedInteger | TypeMap::SignedInteger) => left,
            _ => return,
        }
    }

    pub fn map_json(&mut self, value: &JsonValue) {
        recurse(value, self);

        fn recurse(value: &JsonValue, map: &mut TypeMap) {
            match (value, map) {
                (&JsonValue::Object(ref jm), TypeMap::Object(ref mut tm)) => {
                    for (jk, jv) in jm.iter() {
                        let entry = tm.entry(jk.clone()).or_insert(TypeMap::return_type(jv));
                        recurse(jv, entry);
                    }
                }
                (&JsonValue::Array(ref jm), TypeMap::Array(ref mut tm)) => {
                    for (ji, jv) in jm.iter().enumerate() {
                        let entry = tm.entry(ji).or_insert(TypeMap::return_type(jv));
                        recurse(jv, entry);
                    }
                }
                (&JsonValue::String(ref s), t) => TypeMap::cmp_string(s, t),
                (&JsonValue::Number(ref n), t) => TypeMap::cmp_number(n, t),
                _ => {}
            }
        }
    }
}
