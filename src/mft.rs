#![allow(dead_code)]
use mft::csv::FlatMftEntryWithName;
use std::{
    convert::TryFrom,
    fs::{self, OpenOptions},
    io::Write,
    sync::{Arc, Mutex},
};
type MftParser = mft::MftParser<std::io::BufReader<std::fs::File>>;
use crate::{error::CustomError, job::Task, type_map::IndexPatternObject, type_map::Mapping};

pub struct Parser {
    pub parser: MftParser,
    iter: Vec<FlatMftEntryWithName>,
    //
    data_file: fs::File,
    mapping_ref: Arc<Mutex<Mapping>>,
}

impl TryFrom<&Task> for Parser {
    type Error = CustomError;
    fn try_from(task: &Task) -> Result<Self, Self::Error> {
        std::fs::create_dir_all(format!("{}/{}/", crate::UPLOAD_DIR_ENV, task.job_id))
            .map_err(|e| CustomError::ParserRunError(e.into()))?;
        let data_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!(
                "{}/{}/{}.data",
                crate::UPLOAD_DIR_ENV,
                task.job_id,
                task.id
            ))
            .map_err(|e| CustomError::ParserRunError(e.into()))?;
        let mut parser: MftParser =
            MftParser::from_path(&task.path).map_err(|e| CustomError::ParserRunError(e.into()))?;
        let entries = parser.iter_entries().collect::<Vec<_>>();
        let mut iter = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => iter.push(FlatMftEntryWithName::from_entry(&entry, &mut parser)),
                Err(e) => return Err(CustomError::ParserInitialiseError(e.into())),
            }
        }
        Ok(Self {
            iter,
            parser,
            data_file,
            mapping_ref: task.mapping_ref.clone(),
        })
    }
}
impl Parser {
    pub fn run(&mut self, pattern: IndexPatternObject) -> Result<(), CustomError> {
        //  Iterate over inner parser object
        for entry in &self.iter {
            // Parse entry
            let json =
                serde_json::to_value(entry).map_err(|e| CustomError::ParserRunError(e.into()))?;
            writeln!(&mut self.data_file, "{}", json.to_string())
                .map_err(|e| CustomError::ParserRunError(e.into()))?;
            // Generate type map
            match self.mapping_ref.lock() {
                Ok(mut mapping) => {
                    mapping.map_json(&json, &pattern);
                }
                Err(_err) => {
                    return Err(CustomError::ParserRunError(
                        "Unable to lock mapping mutex reference".into(),
                    ))
                }
            }
        }
        Ok(())
    }
}
