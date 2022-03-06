#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
//
pub mod api;
pub mod elastic;
pub mod error;
pub mod evtx;
pub mod job;
pub mod mft;
pub mod type_map;
pub mod workerpool;

use job::Task;
use std::{
    env, fs,
    io::{self, prelude::*},
    path::PathBuf,
};
// Consts
const UPLOAD_DIR_ENV: &str = "UPLOAD_DIR";
const MONGODB_ADDRESS_ENV: &str = "MONGODB_ADDRESS";
const ELASTIC_USER_ENV: &str = "ELASTIC_USER";
// Env Var Reads
lazy_static! {
    static ref UPLOAD_DIR_PATH: String =
        env::var(UPLOAD_DIR_ENV).unwrap_or_else(|_| "/tmp".to_string());
    static ref MONGODB_ADDRESS: String =
        env::var(MONGODB_ADDRESS_ENV).expect("No Enviroment variable for MONGODB_ADDRESS");
    static ref ELASTIC_USER: String =
        env::var(ELASTIC_USER_ENV).unwrap_or_else(|_| "elastic:changeme".to_string());
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
            [0x45, 0x6c, 0x66, 0x46, 0x69, 0x6c, 0x65, _] => Ok(Self::Evtx),
            _ => match path.extension().map(|ext| ext.to_str().unwrap()) {
                Some("evtx") => Ok(Self::Evtx),
                _ => Ok(Self::None),
            },
        }
    }
}

impl Parser {
    pub fn run_parser(&self, task: &Task) {
        match self {
            Self::Mft => {
                debug!("Creating MFT Parser");
                let mut mft: mft::Parser = TryFrom::try_from(task).unwrap();
                debug!("Running MFT Parser");
                mft.run("pattern".into()).unwrap();
            }
            Self::Evtx => {
                debug!("Creating EVTX Parser");
                let mut evtx: evtx::Parser = TryFrom::try_from(task).unwrap();
                debug!("Running EVTX Parser");
                evtx.run("evtx_{{Event.System.Provider_attributes.Name}}".into())
                    .unwrap();
            }
            _ => panic!("No Parser for this file"),
        }
    }
}
