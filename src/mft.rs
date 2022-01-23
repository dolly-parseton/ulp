#![allow(dead_code)]
use mft::csv::FlatMftEntryWithName;
use std::{fs::OpenOptions, io::Write, path::Path};
type MftParser = mft::MftParser<std::io::BufReader<std::fs::File>>;

pub struct ParserWrapper {
    pub parser: MftParser,
    iter: Vec<FlatMftEntryWithName>,
}

impl ParserWrapper {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, mft::err::Error> {
        let mut parser: MftParser = MftParser::from_path(&path)?;
        let entries = parser.iter_entries().collect::<Vec<_>>();
        let mut iter = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => iter.push(FlatMftEntryWithName::from_entry(&entry, &mut parser)),
                Err(err) => return Err(err),
            }
        }
        Ok(Self { iter, parser })
    }

    pub fn run(&mut self, _targets: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        // let now = std::time::Instant::now();
        // println!("Reading MFT!");
        // let mut value = crate::type_map::Mapping::default();
        // value.set_target(vec!["IsDeleted"]);
        // let mut data = Vec::new();
        let uuid = uuid::Uuid::new_v4();
        let mut file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(format!("{}.json", uuid))?;
        //
        println!("Creating ouput file {}", uuid);

        for entry in self {
            let _json = serde_json::to_value(entry).unwrap();
            write!(&mut file, "{}\n", _json.to_string())?;
            // println!("{}", _json);
            // value.map_json(&json);
        }
        Ok(())
    }
}

impl Iterator for ParserWrapper {
    type Item = FlatMftEntryWithName;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.pop()
    }
}
