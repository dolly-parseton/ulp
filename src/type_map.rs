use crate::error::CustomError;
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};
pub use type_mapping::TypeMap;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct IndexPatternObject {
    pub parts: Vec<(String, bool)>,
}

impl From<&str> for IndexPatternObject {
    fn from(s: &str) -> Self {
        let mut parts = Vec::new();
        for (i, part) in s.split_inclusive(&['{', '}'][..]).enumerate() {
            if part != "{" && part != "}" {
                if i > 0 && i != s.split_inclusive(&['{', '}'][..]).count() - 1 {
                    if s.split_inclusive(&['{', '}'][..]).collect::<Vec<&str>>()[i - 1]
                        .ends_with('{')
                        && s.split_inclusive(&['{', '}'][..]).collect::<Vec<&str>>()[i + 1] == "}"
                    {
                        parts.push((part.trim_end_matches('}').to_string(), true));
                    } else {
                        parts.push((part.trim_end_matches('{').to_string(), false));
                    }
                } else {
                    parts.push((part.trim_end_matches('{').to_string(), false));
                }
            }
        }
        Self { parts }
    }
}

impl IndexPatternObject {
    pub fn generate_index_pattern(&self, data: &serde_json::Value) -> String {
        let mut path = String::new();
        for (key, eval) in self.parts.iter() {
            if *eval {
                match get_value(data, key) {
                    None => path.push_str("NONE"),
                    Some(v) => {
                        use serde_json::Value::*;
                        match v {
                            Array(_) => path.push_str("ARRAY"),
                            Object(_) => path.push_str("OBJECT"),
                            _ => {
                                if let Some(s) = v.as_str() {
                                    path.push_str(s)
                                }
                            }
                        }
                    }
                }
            } else {
                path.push_str(key);
            }
        }
        path
    }
}

fn get_value<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    fn recurse<'a>(keys: &[&str], data: &'a serde_json::Value) -> Option<&'a serde_json::Value> {
        if let Some(key) = keys.get(0) {
            match key.parse::<usize>() {
                Ok(i) => {
                    if let Some(value) = data.get(i) {
                        return recurse(&keys[1..], value);
                    }
                }
                Err(_) => {
                    if let Some(value) = data.get(key) {
                        return recurse(&keys[1..], value);
                    }
                }
            }
        } else {
            return Some(data);
        }
        None
    }
    //
    let keys = key.split('.').collect::<Vec<&str>>();
    recurse(&keys, value)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct Mapping {
    pub map: type_mapping::TypeMap,
    // pub index_pattern: IndexPatternObject,
    pub index_pattern_mappings:
        BTreeMap<String, (type_mapping::TypeMap, Vec<type_mapping::TypeChange>)>, // Key is unique value from delimiter
    //
    pub file_mapping: Vec<ParsedFileStats>,
    pub change_log: Vec<type_mapping::TypeChange>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct ParsedFileStats {
    pub parsed_file_uuid: uuid::Uuid,
    pub source_file_path: PathBuf,
    pub parsed_file_path: PathBuf,
    pub file_size: u64,
    pub file_hash: String,
    pub parser_used: crate::Parser,
}

impl Mapping {
    pub fn add_parsed_file<P: AsRef<Path> + Into<PathBuf>>(
        &mut self,
        job_uuid: uuid::Uuid,
        uuid: uuid::Uuid,
        path: P,
        parser_used: crate::Parser,
    ) -> Result<(), CustomError> {
        use sha2::Digest;
        let mut hash_digest = sha2::Sha256::new();
        let mut file_handle = fs::File::open(&path).map_err(|e| {
            CustomError::StatGenerationError(format!("Failed to grab file handle: {}", e).into())
        })?;
        io::copy(&mut file_handle, &mut hash_digest).map_err(|e| {
            CustomError::StatGenerationError(format!("Failed to copy data to digest: {}", e).into())
        })?;
        let file_hash = format!("{:x}", hash_digest.finalize());
        let file_size = file_handle
            .metadata()
            .map_err(|e| {
                CustomError::StatGenerationError(
                    format!("Failed to read file metadata: {}", e).into(),
                )
            })?
            .len();
        self.file_mapping.push(ParsedFileStats {
            parsed_file_uuid: uuid,
            source_file_path: path.into(),
            // Generate path
            parsed_file_path: fs::canonicalize(format!(
                "{}/{}/{}.data",
                crate::UPLOAD_DIR_ENV,
                job_uuid,
                uuid
            ))
            .map_err(|e| {
                CustomError::StatGenerationError(
                    format!("Failed to canonicalize data file path: {}", e).into(),
                )
            })?,
            file_size,
            file_hash,
            parser_used,
        });
        Ok(())
    }
    pub fn map_json(&mut self, value: &serde_json::Value, index_pattern: &IndexPatternObject) {
        // Update global map
        self.map.map_json(value, &mut self.change_log);
        // Index pattern
        let pattern = index_pattern.generate_index_pattern(value);
        let (map, changes) = self
            .index_pattern_mappings
            .entry(pattern)
            .or_insert((type_mapping::TypeMap::return_type(value), Vec::new()));
        map.map_json(value, changes);
    }
    pub fn cast_json(
        &self,
        value: &mut serde_json::Value,
        index_pattern: Option<&str>,
    ) -> Result<(), CustomError> {
        match index_pattern {
            None => type_casting::cast_to_type_map(value, &self.map)?,
            Some(pattern) => {
                let (map, _) = match self.index_pattern_mappings.get(pattern) {
                    Some(m) => m,
                    None => {
                        return Err(CustomError::TypeCastError(
                            format!(
                            "Attempted to read index_pattern_mappings with key: {}. Does not exist",
                            pattern
                        )
                            .into(),
                        ))
                    }
                };
                type_casting::cast_to_type_map(value, map)?
            }
        }
        Ok(())
    }
}

mod type_mapping {
    use serde_json::Value as JsonValue;
    use std::{
        collections::BTreeMap,
        net::{Ipv4Addr, Ipv6Addr},
    };
    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
    pub struct TypeChange {
        pub path: String,
        pub value: JsonValue,
        pub old_type: TypeMap,
        pub new_type: TypeMap,
    }
    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
    pub enum TypeMap {
        Null,
        Boolean,
        UnsignedInteger,
        SignedInteger,
        Double,
        IPv4,
        IPv6,
        Date,
        String,
        Array(BTreeMap<usize, TypeMap>),
        Object(BTreeMap<String, TypeMap>),
    }
    impl Default for TypeMap {
        fn default() -> Self {
            TypeMap::Object(BTreeMap::new())
        }
    }
    // ------------------------------------------------------------------------------------------------
    // testing
    lazy_static! {
        static ref DATE_REGEX_1: regex::Regex = regex::Regex::new(
            r#"[0-9]{2,4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{3,6}Z"#
        )
        .unwrap();
    }
    // ------------------------------------------------------------------------------------------------
    impl TypeMap {
        fn get_dominant_type(left: &Self, right: &Self) -> Self {
            // Where left is the dominant type and right is the query type
            match (left, right) {
                // left Boolean
                (TypeMap::Boolean, TypeMap::Null) => TypeMap::Boolean,
                (TypeMap::Boolean, r) => r.clone(),
                // (TypeMap::Boolean, TypeMap::UnsignedInteger) => TypeMap::UnsignedInteger, // Catch values where 0 or 1 moving to number
                // (TypeMap::Boolean, TypeMap::SignedInteger) => TypeMap::SignedInteger, // Catch values where 0 or 1 moving to number
                // (TypeMap::Boolean, TypeMap::String) => TypeMap::String, // Catch values where boolean and string
                // (TypeMap::Boolean, TypeMap::IPv4) => TypeMap::String, // Catch values where boolean and string
                // (TypeMap::Boolean, TypeMap::IPv6) => TypeMap::String, // Catch values where boolean and string
                // left UnsignedInteger
                (TypeMap::UnsignedInteger, TypeMap::Null) => TypeMap::UnsignedInteger,
                (TypeMap::UnsignedInteger, TypeMap::Boolean) => TypeMap::UnsignedInteger, // Cast to bool to unsigned int
                (TypeMap::UnsignedInteger, r) => r.clone(),
                // (TypeMap::UnsignedInteger, TypeMap::String) => TypeMap::String, // !! This is to catch fields with hex and strings causing dominant type panic
                // left SignedInteger
                (TypeMap::SignedInteger, TypeMap::Null) => TypeMap::SignedInteger,
                (TypeMap::SignedInteger, TypeMap::Boolean) => TypeMap::SignedInteger,
                (TypeMap::SignedInteger, TypeMap::UnsignedInteger) => TypeMap::SignedInteger, // Cast unsigned int to signed int
                (TypeMap::SignedInteger, r) => r.clone(),
                // (TypeMap::SignedInteger, TypeMap::String) => TypeMap::String, // !! This is to catch fields with hex and strings causing dominant type panic
                // left Double
                (TypeMap::Double, TypeMap::Null) => TypeMap::Double,
                (TypeMap::Double, TypeMap::Boolean) => TypeMap::Double,
                (TypeMap::Double, TypeMap::UnsignedInteger) => TypeMap::Double, // Cast unsigned int to signed int
                (TypeMap::Double, TypeMap::SignedInteger) => TypeMap::Double, // Cast signed int to float
                (TypeMap::Double, r) => r.clone(),
                // left IPv4 - Start of impossible conditions
                (TypeMap::IPv4, TypeMap::Null) => TypeMap::IPv4,
                // Defaulting to string for complex types
                (TypeMap::IPv4, TypeMap::Boolean) => TypeMap::String,
                (TypeMap::IPv4, TypeMap::UnsignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv4, TypeMap::SignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv4, TypeMap::Double) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv4, r) => r.clone(),
                // left IPv6
                (TypeMap::IPv6, TypeMap::Null) => TypeMap::IPv6,
                // Defaulting to string for complex types
                (TypeMap::IPv6, TypeMap::Boolean) => TypeMap::String,
                (TypeMap::IPv6, TypeMap::UnsignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv6, TypeMap::SignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv6, TypeMap::Double) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv6, TypeMap::IPv4) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv6, r) => r.clone(),
                // left Date
                (TypeMap::Date, TypeMap::Null) => TypeMap::Date,
                // Defaulting to string for complex types
                (TypeMap::Date, TypeMap::Boolean) => TypeMap::String,
                (TypeMap::Date, TypeMap::UnsignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::Date, TypeMap::SignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::Date, TypeMap::Double) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::Date, TypeMap::IPv4) => TypeMap::String,
                (TypeMap::Date, TypeMap::IPv6) => TypeMap::String,
                // left String
                (TypeMap::String, TypeMap::Null) => TypeMap::String,
                (TypeMap::String, TypeMap::Boolean) => TypeMap::String,
                (TypeMap::String, TypeMap::UnsignedInteger) => TypeMap::String,
                (TypeMap::String, TypeMap::SignedInteger) => TypeMap::String,
                (TypeMap::String, TypeMap::Double) => TypeMap::String,
                (TypeMap::String, TypeMap::IPv4) => TypeMap::String,
                (TypeMap::String, TypeMap::IPv6) => TypeMap::String,
                (TypeMap::String, TypeMap::Date) => TypeMap::String,
                (TypeMap::String, TypeMap::Array(_)) | (TypeMap::Array(_), TypeMap::String) => {
                    TypeMap::String
                }
                (TypeMap::String, TypeMap::Object(_)) | (TypeMap::Object(_), TypeMap::String) => {
                    TypeMap::String
                }
                // left Array
                (TypeMap::Array(left_array), TypeMap::Null) => TypeMap::Array(left_array.clone()),
                (TypeMap::Array(left_array), TypeMap::Array(right_array)) => {
                    let mut dominant_array = BTreeMap::new();
                    for (key, value) in left_array {
                        if let Some(right_value) = right_array.get(key) {
                            dominant_array
                                .insert(*key, TypeMap::get_dominant_type(value, right_value));
                        } else {
                            dominant_array.insert(*key, value.clone());
                        }
                    }
                    TypeMap::Array(dominant_array)
                }
                (TypeMap::Array(_), _) => unimplemented!(),
                // left Object
                (TypeMap::Object(left_object), TypeMap::Null) => {
                    TypeMap::Object(left_object.clone())
                }
                (TypeMap::Object(left_object), TypeMap::Object(right_object)) => {
                    let mut dominant_object = BTreeMap::new();
                    for (key, value) in left_object {
                        if let Some(right_value) = right_object.get(key) {
                            dominant_object.insert(
                                key.to_string(),
                                TypeMap::get_dominant_type(value, right_value),
                            );
                        } else {
                            dominant_object.insert(key.to_string(), value.clone());
                        }
                    }
                    TypeMap::Object(dominant_object)
                }
                (TypeMap::Object(_left_object), _) => unimplemented!(),
                // Complex type cases
                // (TypeMap::IPv4, TypeMap::String) => TypeMap::String,
                // (TypeMap::IPv6, TypeMap::String) => TypeMap::String,
                (TypeMap::Date, TypeMap::String) => TypeMap::String,
                // Null cases
                // left Null
                (TypeMap::Null, TypeMap::Null) => TypeMap::Null,
                (TypeMap::Null, r) => r.clone(),
                // Deal with other edge cases here
                (l, r) => {
                    if l == r {
                        return l.clone();
                    }
                    error!("Unimplemented type clash: {:?}", (l, r));
                    unimplemented!()
                }
            }
        }
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
            if let Some(v) = value.as_u64() {
                match v {
                    0 | 1 => Self::Boolean,
                    _ => Self::UnsignedInteger,
                }
            } else if let Some(v) = value.as_i64() {
                match v {
                    0 | 1 => Self::Boolean,
                    _ => Self::SignedInteger,
                }
            } else {
                Self::Double
            }
        }
        pub fn map_string(value: &str) -> Self {
            if value.to_ascii_lowercase() == "true" || value.to_ascii_lowercase() == "false" {
                Self::Boolean
            // Push DTA code here, link the library for parsing variable types from a dictionary
            } else if let Some(hex) = value.to_ascii_lowercase().strip_prefix("0x") {
                match u64::from_str_radix(hex, 16).is_ok() {
                    true => Self::UnsignedInteger,
                    false => Self::String,
                }
            } else if chrono::DateTime::parse_from_rfc3339(value).is_ok()
                || chrono::DateTime::parse_from_rfc2822(value).is_ok()
                || chrono::DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.6fZ").is_ok()
                || chrono::DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.3fZ").is_ok()
            {
                Self::Date
            } else if value.parse::<Ipv4Addr>().is_ok() {
                Self::IPv4
            } else if value.parse::<Ipv6Addr>().is_ok() {
                Self::IPv6
            } else {
                Self::String
            }
        }
        pub fn map_json(&mut self, value: &JsonValue, changes: &mut Vec<TypeChange>) {
            recurse(None, value, self, changes);
            fn recurse(
                path: Option<String>,
                value: &JsonValue,
                map: &mut TypeMap,
                changes: &mut Vec<TypeChange>,
            ) {
                let p = match path {
                    Some(p) => p,
                    None => "".to_string(),
                };
                match (value, map) {
                    // Handle recursion on object collisions
                    (&JsonValue::Object(ref jm), TypeMap::Object(ref mut tm)) => {
                        for (jk, jv) in jm.iter() {
                            let entry = tm
                                .entry(jk.clone())
                                .or_insert_with(|| TypeMap::return_type(jv));
                            recurse(Some(format!("{}.{}", p, jk)), jv, entry, changes);
                        }
                    }
                    // Handle recursion on array collisions
                    // - TODO determine how to handle arrays with mutliple types (left to right dominance check on change or in post?)
                    (&JsonValue::Array(ref jm), TypeMap::Array(ref mut tm)) => {
                        for (ji, jv) in jm.iter().enumerate() {
                            let entry = tm.entry(ji).or_insert_with(|| TypeMap::return_type(jv));
                            recurse(Some(format!("{}.{}", p, ji)), jv, entry, changes);
                        }
                    }
                    (v, t) => {
                        let left = TypeMap::return_type(v);
                        debug!("Change {:?}", (&v, &t));
                        let new_right = TypeMap::get_dominant_type(&left, t);
                        if &left != t && left != TypeMap::Null && &new_right != t {
                            debug!("Prev {:?}", (&v, &t));
                            debug!("Change {:?} to {:?}", &t, &new_right);
                            changes.push(TypeChange {
                                path: p.strip_prefix('.').unwrap().to_string(), // This unwrap will always succeed
                                old_type: t.clone(),
                                new_type: new_right.clone(),
                                value: v.clone(),
                            });
                        }
                        *t = new_right;
                    }
                }
            }
        }
    }
}

mod type_casting {
    use super::type_mapping::TypeMap;
    use crate::error::CustomError;
    use serde_json::Value as JsonValue;
    pub fn cast_to_type_map(value: &mut JsonValue, type_map: &TypeMap) -> Result<(), CustomError> {
        // Eq
        match (value, type_map) {
            (JsonValue::Null, TypeMap::Null) => Ok(()),
            (JsonValue::Bool(_), TypeMap::Boolean) => Ok(()),
            (JsonValue::Number(_), TypeMap::UnsignedInteger) => Ok(()),
            (JsonValue::Number(_), TypeMap::SignedInteger) => Ok(()),
            (JsonValue::Number(_), TypeMap::Double) => Ok(()),
            (JsonValue::String(_), TypeMap::String) => Ok(()),
            (JsonValue::String(_), TypeMap::Date) => Ok(()),
            (JsonValue::String(_), TypeMap::IPv4) => Ok(()),
            (JsonValue::String(_), TypeMap::IPv6) => Ok(()),
            // Not Eq
            (v, t) => {
                match (v, t) {
                    (JsonValue::Array(array), TypeMap::Array(ref tm)) => {
                        for (i, v) in array.iter_mut().enumerate() {
                            match tm.get(&i) {
                                Some(entry) => cast_to_type_map(v, entry)?,
                                None => return Err(CustomError::TypeCastError("Attempted to cast a field value to a type no mapping exists for.".into())),
                            }
                        }
                        Ok(())
                    }
                    (JsonValue::Object(object), TypeMap::Object(ref tm)) => {
                        for (k, v) in object.iter_mut() {
                            match  tm.get(k) {
                                Some(entry) => cast_to_type_map(v, entry)?,
                                None => return Err(CustomError::TypeCastError("Attempted to cast a field value to a type no mapping exists for.".into())),
                            }
                        }
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    #[test]
    fn index_pattern_test() {
        let pattern = super::IndexPatternObject::from("{{x.y}}_aaa_{{a.b}}_bbb");
        println!("{:?}", pattern);
        let data = json!({
            "x": {
                "y": "apple"
            },
            "a": {
                "b": "pear"
            }
        });
        assert_eq!(pattern.generate_index_pattern(&data), "apple_aaa_pear_bbb");
    }
}
