#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate mongod;

pub mod api_routes;
pub mod mongodb;
pub mod queue;
pub mod type_map;

use api_routes::{cleanup, download, upload};
use std::env;
// Consts
const UPLOAD_DIR_ENV: &'static str = "UPLOAD_DIR";
const MONGODB_ADDRESS_ENV: &'static str = "MONGODB_ADDRESS";
// Env Var Reads
lazy_static! {
    static ref UPLOAD_DIR_PATH: String =
        env::var(UPLOAD_DIR_ENV).expect("No Enviroment variable for UPLOAD_DIR");
    static ref MONGODB_ADDRESS: String =
        env::var(MONGODB_ADDRESS_ENV).expect("No Enviroment variable for MONGODB_ADDRESS");
}
