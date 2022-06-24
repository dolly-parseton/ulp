use crate::error::CustomError;
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};
use type_casting::{cast_value, merge, Types};

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
    pub map: Types,
    // pub index_pattern: IndexPatternObject,
    pub index_pattern_mappings: BTreeMap<String, Types>, // Key is unique value from delimiter
    //
    pub file_mapping: Vec<ParsedFileStats>,
    // pub change_log: Vec<()>,
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
        let value_type = Types::get_type(value);
        merge(&mut self.map, value_type);
        // self.map = new_map;
        // Index pattern
        let pattern = index_pattern.generate_index_pattern(value);
        match self.index_pattern_mappings.remove(&pattern) {
            Some(mut index_map) => {
                merge(&mut index_map, Types::get_type(value));
                self.index_pattern_mappings.insert(pattern, index_map);
            }
            None => {
                self.index_pattern_mappings
                    .insert(pattern, Types::get_type(value));
            }
        }
    }
    pub fn cast_json(
        &self,
        value: serde_json::Value,
        index_pattern: Option<&str>,
    ) -> Result<serde_json::Value, CustomError> {
        match index_pattern {
            None => cast_value(&self.map, value).map_err(|e| {
                CustomError::TypeCastError(format!("Failed to cast type, {:?}", e).into())
            }),
            Some(pattern) => {
                let map = self.index_pattern_mappings.get(pattern).ok_or_else(|| {
                    CustomError::TypeCastError(
                        format!(
                            "Attempted to read index_pattern_mappings with key: {}. Does not exist",
                            pattern
                        )
                        .into(),
                    )
                })?;
                cast_value(map, value).map_err(|e| {
                    CustomError::TypeCastError(format!("Failed to cast type, {:?}", e).into())
                })
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
