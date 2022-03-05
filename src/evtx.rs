#![allow(dead_code)]
use evtx::ParserSettings;
use std::{
    convert::TryFrom,
    error,
    fs::{self, OpenOptions},
    io::Write,
    sync::{Arc, Mutex},
};
type EvtxParser = evtx::EvtxParser<std::fs::File>;
use crate::{job::Task, type_map::IndexPatternObject, type_map::Mapping};

pub struct Parser {
    pub parser: EvtxParser,
    //
    data_file: fs::File,
    mapping_ref: Arc<Mutex<Mapping>>,
}

impl TryFrom<&Task> for Parser {
    type Error = Box<dyn error::Error>;
    fn try_from(task: &Task) -> Result<Self, Self::Error> {
        std::fs::create_dir_all(format!("{}/{}/", crate::UPLOAD_DIR_ENV, task.job_id)).unwrap();
        let data_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!(
                "{}/{}/{}.data",
                crate::UPLOAD_DIR_ENV,
                task.job_id,
                task.id
            ))?;
        let parser = evtx::EvtxParser::from_path(&task.path)?
            .with_configuration(ParserSettings::new().separate_json_attributes(true));
        Ok(Self {
            parser,
            data_file,
            mapping_ref: task.mapping_ref.clone(),
        })
    }
}
impl Parser {
    pub fn run(
        &mut self,
        pattern: IndexPatternObject,
    ) -> Result<(), Box<dyn std::error::Error + '_>> {
        //  Iterate over inner parser object
        for record in self.parser.records_json_value() {
            match record {
                Ok(json) => {
                    writeln!(&mut self.data_file, "{}", json.data.to_string())?;
                    match self.mapping_ref.lock() {
                        Ok(mut mapping) => {
                            mapping.map_json(&json.data, &pattern);
                        }
                        Err(_err) => return Err("err".into()),
                    }
                }
                Err(e) => {
                    error!("Unable to convert serialsied record to json: {}", e);
                    panic!("Unable to convert serialsied record to json: {}", e);
                }
            };
        }
        Ok(())
    }
}
