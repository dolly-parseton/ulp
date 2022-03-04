use glob::glob;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    {io, io::prelude::*},
};
use uuid::Uuid;

#[derive(serde::Serialize, Debug, Clone, Default)]
pub struct Job {
    pub id: Uuid,
    pub paths: Vec<PathBuf>,
    pub status: Status,
}

impl From<&str> for Job {
    fn from(path_glob: &str) -> Self {
        // Test file_path for parser_type
        let mut paths = Vec::new();
        for entry in glob(path_glob).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => paths.push(path),
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Job {
            id: Uuid::new_v4(),
            paths,
            status: Status::default(),
        }
    }
}

#[derive(serde::Serialize, Debug, Clone, Default)]
pub struct Task {
    pub job_id: Uuid,
    pub id: Uuid,
    pub path: PathBuf,
}

impl Iterator for Job {
    type Item = Task;
    fn next(&mut self) -> Option<Self::Item> {
        match self.paths.pop() {
            Some(path) => Some(Task {
                job_id: self.id,
                id: Uuid::new_v4(),
                path,
            }),
            None => None,
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
pub enum Status {
    Pending,
    Done,
}

impl Default for Status {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IndexPatternFile(Vec<IndexPatternObject>);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub scope: String,
    pub total_entries: usize,
    pub total_index_patterns: usize,
    pub index_pattern_counts: HashMap<String, usize>,
}

pub struct Data {
    pub inner: serde_json::Value,
}

impl Data {
    pub fn generate_index_pattern(&self, index_pattern: &IndexPatternObject) -> String {
        let mut path = String::new();
        for (key, eval) in index_pattern.parts.iter() {
            if *eval {
                match self.get_value(key) {
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

    fn get_value(&self, key: &str) -> Option<&serde_json::Value> {
        fn recurse<'a>(
            keys: &[&str],
            data: &'a serde_json::Value,
        ) -> Option<&'a serde_json::Value> {
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
        recurse(&keys, &self.inner)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    #[test]
    fn index_pattern_test() {
        let pattern = super::IndexPatternObject::from("{{x.y}}_aaa_{{a.b}}_bbb");
        println!("{:?}", pattern);
        let data = crate::job::Data {
            inner: json!({
                "x": {
                    "y": "apple"
                },
                "a": {
                    "b": "pear"
                }
            }),
        };
        assert_eq!(data.generate_index_pattern(&pattern), "apple_aaa_pear_bbb");
    }
}
