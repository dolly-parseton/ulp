use mft::{csv::FlatMftEntryWithName, entry::MftEntry};
use std::path::Path;

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
}

impl Iterator for ParserWrapper {
    type Item = FlatMftEntryWithName;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.pop()
    }
}
