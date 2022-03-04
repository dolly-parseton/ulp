#[macro_use]
extern crate lazy_static;
extern crate log;
//
pub mod api;
pub mod job;
pub mod mft;
pub mod type_map;
pub mod workerpool;

pub use type_map::Mapping;

use std::{
    env, fs,
    io::{self, prelude::*},
    path::{Path, PathBuf},
};
// Consts
const UPLOAD_DIR_ENV: &str = "UPLOAD_DIR";
const MONGODB_ADDRESS_ENV: &str = "MONGODB_ADDRESS";
// Env Var Reads
lazy_static! {
    static ref UPLOAD_DIR_PATH: String =
        env::var(UPLOAD_DIR_ENV).expect("No Enviroment variable for UPLOAD_DIR");
    static ref MONGODB_ADDRESS: String =
        env::var(MONGODB_ADDRESS_ENV).expect("No Enviroment variable for MONGODB_ADDRESS");
}

#[derive(serde::Serialize, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Parser {
    Evtx,
    Mft,
    None,
}

impl Default for Parser {
    fn default() -> Self {
        Self::None
    }
}

impl TryFrom<&PathBuf> for Parser {
    type Error = io::Error;
    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let mut file = fs::File::open(path)?;
        let mut buffer: [u8; 8] = [0; 8];
        file.read_exact(&mut buffer)?;
        match &buffer[..] {
            [0x46, 0x49, 0x4c, 0x45, 0x30, _, _, _] => Ok(Self::Mft),
            _ => Ok(Self::None),
        }
    }
}

impl Parser {
    pub fn run_parser<P: AsRef<Path>>(&self, _path: P) -> () {
        match self {
            Self::Mft => {
                println!("Running MFT Parser");
            }
            _ => panic!("No Parser for this file"),
        }
    }
}
