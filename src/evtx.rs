#![allow(dead_code)]
use evtx::ParserSettings;
use std::{
    convert::TryFrom,
    fs::{self, OpenOptions},
    io::Write,
    sync::{Arc, Mutex},
};
type EvtxParser = evtx::EvtxParser<std::fs::File>;
use crate::{error::CustomError, job::Task, type_map::IndexPatternObject, type_map::Mapping};

pub struct Parser {
    pub parser: EvtxParser,
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
        let parser = evtx::EvtxParser::from_path(&task.path)
            .map_err(|e| CustomError::ParserRunError(e.into()))?
            .with_configuration(ParserSettings::new().separate_json_attributes(true));
        Ok(Self {
            parser,
            data_file,
            mapping_ref: task.mapping_ref.clone(),
        })
    }
}
impl Parser {
    pub fn run(&mut self, pattern: IndexPatternObject) -> Result<(), CustomError> {
        //  Iterate over inner parser object
        for record in self.parser.records_json_value() {
            // Read in json object
            let json = record.map_err(|e| CustomError::ParserRunError(e.into()))?;
            // Write to file
            writeln!(&mut self.data_file, "{}", json.data.to_string())
                .map_err(|e| CustomError::ParserRunError(e.into()))?;
            // Generate type mapping
            match self.mapping_ref.lock() {
                Ok(mut mapping) => {
                    mapping.map_json(&json.data, &pattern);
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
