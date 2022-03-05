#![allow(dead_code)]
use mft::csv::FlatMftEntryWithName;
use std::{
    convert::TryFrom,
    error,
    fs::{self, OpenOptions},
    io::Write,
    sync::{Arc, Mutex},
};
type MftParser = mft::MftParser<std::io::BufReader<std::fs::File>>;
use crate::{job::Task, type_map::IndexPatternObject, type_map::Mapping};

pub struct Parser {
    pub parser: MftParser,
    iter: Vec<FlatMftEntryWithName>,
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
        println!("Creating MFT Parser inner");
        let mut parser: MftParser = MftParser::from_path(&task.path)?;
        let entries = parser.iter_entries().collect::<Vec<_>>();
        let mut iter = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => iter.push(FlatMftEntryWithName::from_entry(&entry, &mut parser)),
                Err(err) => return Err(err.into()),
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
    pub fn run(
        &mut self,
        pattern: IndexPatternObject,
    ) -> Result<(), Box<dyn std::error::Error + '_>> {
        //  Iterate over inner parser object
        for entry in &self.iter {
            let json = serde_json::to_value(entry).unwrap();
            writeln!(&mut self.data_file, "{}", json.to_string())?;
            match self.mapping_ref.lock() {
                Ok(mut mapping) => {
                    mapping.map_json(&json, &pattern);
                }
                Err(err) => return Err("err".into()),
            }
        }
        Ok(())
    }
}
