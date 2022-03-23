use crate::{error::CustomError, type_map::Mapping};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant, // {io, io::prelude::*},
};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub paths: Vec<PathBuf>,
    pub status: Status,
    pub mapping: Arc<Mutex<Mapping>>,
    pub sent: Arc<Mutex<Vec<(Uuid, PathBuf)>>>,
    #[serde(skip)]
    pub processed: Vec<Task>,
    #[serde(with = "approx_instant")]
    pub completed: Instant,
}

mod approx_instant {
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Instant, SystemTime};

    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let system_now = SystemTime::now();
        let instant_now = Instant::now();
        let approx = system_now - (instant_now - *instant);
        approx.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        let de = SystemTime::deserialize(deserializer)?;
        let system_now = SystemTime::now();
        let instant_now = Instant::now();
        let duration = system_now.duration_since(de).map_err(Error::custom)?;
        let approx = instant_now - duration;
        Ok(approx)
    }
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
                completed: Instant::now(),
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

impl Task {
    pub fn add_parsed_file_stats(&self, parser: crate::Parser) -> Result<(), CustomError> {
        let mut mapping = self.mapping_ref.lock().unwrap();
        mapping.add_parsed_file(self.job_id, self.id, &self.path, parser)?;
        Ok(())
    }
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
                        sent.push((task_id, path.clone()));
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Status {
    Pending,
    Done,
}

impl Default for Status {
    fn default() -> Self {
        Self::Pending
    }
}
