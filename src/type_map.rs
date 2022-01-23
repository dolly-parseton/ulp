use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Mapping {
    pub map: type_mapping::TypeMap,
    pub target: Option<Target>,
    pub targeted_mapping: BTreeMap<String, (type_mapping::TypeMap, Vec<type_mapping::TypeChange>)>, // Key is unique value from delimiter
    pub change_log: Vec<type_mapping::TypeChange>,
}

impl Mapping {
    pub fn set_target(&mut self, path_parts: Vec<&str>) {
        self.target = Some(Target {
            parts: path_parts.into_iter().map(|s| s.to_string()).collect(),
            delimiter: ".".to_string(),
            unique_variations: Vec::new(),
        });
    }
    pub fn map_json(&mut self, value: &serde_json::Value) {
        // Update global map
        self.map.map_json(value, &mut self.change_log);
        // Get target field
        if let Some(target) = self.target.as_mut() {
            if let Some(key) = recurse_fields(value, &target.parts, 0) {
                let key_s = key.to_string();
                // Might want to compare performance on push + dedup
                if !target.unique_variations.contains(&key_s) {
                    target.unique_variations.push(key_s.clone());
                }
                let (map, changes) = self
                    .targeted_mapping
                    .entry(key_s)
                    .or_insert((type_mapping::TypeMap::return_type(value), Vec::new()));
                map.map_json(value, changes);
            }
        }

        //
        fn recurse_fields(
            value: &serde_json::Value,
            target: &[String],
            target_index: usize,
        ) -> Option<serde_json::Value> {
            if target_index == target.len() {
                return Some(value.clone());
            }
            if let Some(value) = value.get(&target[target_index]) {
                return recurse_fields(value, target, target_index + 1);
            }
            None
        }
    }
    pub fn cast_json(&self, value: &mut serde_json::Value) {
        type_casting::cast_to_type_map(value, &self.map);
    }
}

#[derive(Debug, Clone, Default)]
pub struct Target {
    pub parts: Vec<String>,
    pub delimiter: String,
    pub unique_variations: Vec<String>,
}

mod type_mapping {
    use serde_json::Value as JsonValue;
    use std::{
        collections::BTreeMap,
        net::{Ipv4Addr, Ipv6Addr},
    };
    #[derive(Debug, Clone)]
    pub struct TypeChange {
        pub path: String,
        pub value: JsonValue,
        pub old_type: TypeMap,
        pub new_type: TypeMap,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
                // left UnsignedInteger
                (TypeMap::UnsignedInteger, TypeMap::Null) => TypeMap::UnsignedInteger,
                (TypeMap::UnsignedInteger, TypeMap::Boolean) => TypeMap::UnsignedInteger, // Cast to bool to unsigned int
                // left SignedInteger
                (TypeMap::SignedInteger, TypeMap::Null) => TypeMap::SignedInteger,
                (TypeMap::SignedInteger, TypeMap::Boolean) => TypeMap::SignedInteger,
                (TypeMap::SignedInteger, TypeMap::UnsignedInteger) => TypeMap::SignedInteger, // Cast unsigned int to signed int
                // left Double
                (TypeMap::Double, TypeMap::Null) => TypeMap::Double,
                (TypeMap::Double, TypeMap::Boolean) => TypeMap::Double,
                (TypeMap::Double, TypeMap::UnsignedInteger) => TypeMap::Double, // Cast unsigned int to signed int
                (TypeMap::Double, TypeMap::SignedInteger) => TypeMap::Double, // Cast signed int to float
                // left IPv4 - Start of impossible conditions
                (TypeMap::IPv4, TypeMap::Null) => TypeMap::IPv4,
                // Defaulting to string for complex types
                (TypeMap::IPv4, TypeMap::Boolean) => TypeMap::String,
                (TypeMap::IPv4, TypeMap::UnsignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv4, TypeMap::SignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv4, TypeMap::Double) => TypeMap::String, // Casting could maybe be attempted here in practice
                // left IPv6
                (TypeMap::IPv6, TypeMap::Null) => TypeMap::IPv6,
                // Defaulting to string for complex types
                (TypeMap::IPv6, TypeMap::Boolean) => TypeMap::String,
                (TypeMap::IPv6, TypeMap::UnsignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv6, TypeMap::SignedInteger) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv6, TypeMap::Double) => TypeMap::String, // Casting could maybe be attempted here in practice
                (TypeMap::IPv6, TypeMap::IPv4) => TypeMap::String, // Casting could maybe be attempted here in practice
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
                (TypeMap::IPv4, TypeMap::String) => TypeMap::String,
                (TypeMap::IPv6, TypeMap::String) => TypeMap::String,
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
                    println!("{:?}", (l, r));
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
            if value.is_u64() {
                Self::UnsignedInteger
            } else if value.is_i64() {
                Self::SignedInteger
            } else {
                Self::Double
            }
        }
        pub fn map_string(value: &str) -> Self {
            if value.to_ascii_lowercase() == "true" || value.to_ascii_lowercase() == "false" {
                Self::Boolean
            // Push DTA code here, link the library for parsing variable types from a dictionary
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
                        // println!("Change {:?}", (&v, &t));
                        let new_right = TypeMap::get_dominant_type(&left, t);
                        if &left != t && left != TypeMap::Null && &new_right != t {
                            println!("Prev {:?}", (&v, &t));
                            println!("Change {:?} to {:?}", &t, &new_right);
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
    use serde_json::Value as JsonValue;
    pub fn cast_to_type_map(value: &mut JsonValue, type_map: &TypeMap) {
        // Eq
        match (value, type_map) {
            (JsonValue::Null, TypeMap::Null) => (),
            (JsonValue::Bool(_), TypeMap::Boolean) => (),
            (JsonValue::Number(_), TypeMap::UnsignedInteger) => (),
            (JsonValue::Number(_), TypeMap::SignedInteger) => (),
            (JsonValue::Number(_), TypeMap::Double) => (),
            (JsonValue::String(_), TypeMap::String) => (),
            (JsonValue::String(_), TypeMap::Date) => (),
            (JsonValue::String(_), TypeMap::IPv4) => (),
            (JsonValue::String(_), TypeMap::IPv6) => (),
            // Not Eq
            (v, t) => match (v, t) {
                (JsonValue::Array(array), TypeMap::Array(ref tm)) => {
                    for (i, v) in array.iter_mut().enumerate() {
                        let entry = tm.get(&i).unwrap();
                        cast_to_type_map(v, entry);
                    }
                }
                (JsonValue::Object(object), TypeMap::Object(ref tm)) => {
                    for (k, v) in object.iter_mut() {
                        let entry = tm.get(k).unwrap();
                        cast_to_type_map(v, entry);
                    }
                }
                _ => (),
            },
        }
    }
}
