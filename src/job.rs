use crate::{error::CustomError, type_map::Mapping};
use glob::glob;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant, // {io, io::prelude::*},
};
use uuid::Uuid;

#[derive(serde::Serialize, Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub paths: Vec<PathBuf>,
    pub processed: Vec<Task>,
    pub status: Status,
    #[serde(skip)]
    pub started: Instant,
    #[serde(skip)]
    pub mapping: Arc<Mutex<Mapping>>,
    #[serde(skip)]
    pub sent: Arc<Mutex<Vec<Uuid>>>,
}

impl Job {
    pub fn from_glob(path_glob: &str) -> Option<Self> {
        // Test file_path for parser_type
        let mut paths = Vec::new();
        for entry in glob(path_glob).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => paths.push(path),
                Err(e) => eprintln!("{:?}", e),
            }
        }
        match paths.is_empty() {
            true => None,
            false => Some(Self {
                id: Uuid::new_v4(),
                paths,
                sent: Arc::new(Mutex::new(Vec::new())),
                processed: Vec::new(),
                status: Status::default(),
                started: Instant::now(),
                mapping: Arc::new(Mutex::new(Mapping::default())),
            }),
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct Task {
    pub job_id: Uuid,
    pub id: Uuid,
    pub path: PathBuf,
    //
    #[serde(skip)]
    pub mapping_ref: Arc<Mutex<Mapping>>,
}

impl Iterator for Job {
    type Item = Result<Task, CustomError>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.paths.pop() {
            Some(path) => {
                // Track iterated tasks
                let task_id = Uuid::new_v4();
                match self.sent.lock() {
                    Ok(mut sent) => {
                        sent.push(task_id);
                    }
                    Err(e) => {
                        return Some(Err(CustomError::ParserRunError(
                            format!("Unable to lock 'sent' Mutex in Job. {}", e).into(),
                        )))
                    }
                }
                Some(Ok(Task {
                    job_id: self.id,
                    id: task_id,
                    path,
                    mapping_ref: self.mapping.clone(),
                }))
            }
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
pub struct Stats {
    pub scope: String,
    pub total_entries: usize,
    pub total_index_patterns: usize,
    pub index_pattern_counts: HashMap<String, usize>,
}

// pub struct Data {
//     pub inner: serde_json::Value,
// }

// impl Data {
//     pub fn generate_index_pattern(&self, index_pattern: &IndexPatternObject) -> String {
//         let mut path = String::new();
//         for (key, eval) in index_pattern.parts.iter() {
//             if *eval {
//                 match self.get_value(key) {
//                     None => path.push_str("NONE"),
//                     Some(v) => {
//                         use serde_json::Value::*;
//                         match v {
//                             Array(_) => path.push_str("ARRAY"),
//                             Object(_) => path.push_str("OBJECT"),
//                             _ => {
//                                 if let Some(s) = v.as_str() {
//                                     path.push_str(s)
//                                 }
//                             }
//                         }
//                     }
//                 }
//             } else {
//                 path.push_str(key);
//             }
//         }
//         path
//     }

//     fn get_value(&self, key: &str) -> Option<&serde_json::Value> {
//         fn recurse<'a>(
//             keys: &[&str],
//             data: &'a serde_json::Value,
//         ) -> Option<&'a serde_json::Value> {
//             if let Some(key) = keys.get(0) {
//                 match key.parse::<usize>() {
//                     Ok(i) => {
//                         if let Some(value) = data.get(i) {
//                             return recurse(&keys[1..], value);
//                         }
//                     }
//                     Err(_) => {
//                         if let Some(value) = data.get(key) {
//                             return recurse(&keys[1..], value);
//                         }
//                     }
//                 }
//             } else {
//                 return Some(data);
//             }
//             None
//         }
//         //
//         let keys = key.split('.').collect::<Vec<&str>>();
//         recurse(&keys, &self.inner)
//     }
// }
