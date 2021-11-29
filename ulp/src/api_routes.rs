use rocket::data::{Data, ToByteUnit};
use rocket::response::Debug;
use rocket::tokio::fs::File;

use std::{
    fs::{read_dir, remove_file},
    path::Path,
};

pub fn cleanup() {
    for entry in
        read_dir(crate::UPLOAD_DIR_PATH.as_str()).expect("Read Directory on cleanup() failed")
    {
        let entry = entry.expect("Entry in cleanup() failed");
        let path = entry.path();
        println!("{:?}", path);
        if path.is_file() {
            remove_file(path).expect("Remove File on cleanup() failed");
        }
    }
}

#[get("/download/<uuid>")]
pub async fn download(uuid: &str) -> Option<File> {
    let filename = Path::new(crate::UPLOAD_DIR_PATH.as_str()).join(&uuid);
    // let filename = format!("./ulp/.upload/{uuid}", uuid = uuid);
    File::open(&filename).await.ok()
}

#[post("/upload", data = "<paste>")]
pub async fn upload(paste: Data<'_>) -> Result<String, Debug<std::io::Error>> {
    let uuid = uuid::Uuid::new_v4().to_string();
    let filename = Path::new(crate::UPLOAD_DIR_PATH.as_str()).join(&uuid);
    let url = format!(
        "{host}/{uuid}\n",
        host = "http://localhost:8000",
        uuid = &uuid
    );
    // Write the paste out, limited to 128KiB, and return the URL.
    paste.open(4.gibibytes()).into_file(filename).await?;
    Ok(url)
}
