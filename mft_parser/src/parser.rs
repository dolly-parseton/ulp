use mft::{csv::FlatMftEntryWithName, entry::MftEntry};
use std::path::Path;

type MftParser = mft::MftParser<std::io::BufReader<std::fs::File>>;

pub fn get_parser<P: AsRef<Path>>(path: P) -> Result<MftParser, mft::err::Error> {
    MftParser::from_path(&path)
}
