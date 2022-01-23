#[macro_use]
extern crate lazy_static;
extern crate log;
//
pub mod api;
pub mod job;
pub mod mft;
pub mod queue;
pub mod type_map;

pub use type_map::Mapping;

use std::env;
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
