extern crate serde;
extern crate serde_json;

use ulp::type_map::Mapping;

mod parser;

#[macro_use]
extern crate mongod;

#[derive(Bson, Mongo)]
#[mongo(collection = "parsed", field, filter, update)]
pub struct ParsedData {
    pub index_str: String,
    pub data: Vec<u8>,
}

fn main() {
    let now = std::time::Instant::now();
    let client = mongod::blocking::ClientBuilder::new()
        .uri("mongodb://root:example@localhost:27017/")
        .database("mft")
        .build()
        .unwrap();
    println!("Reading MFT!");
    let mut value = Mapping::default();
    // value.set_target(vec!["IsDeleted"]);
    let mut first = true;
    let parser = parser::ParserWrapper::from_path("./.test_data/$MFT").unwrap();
    let mut data = Vec::new();
    for entry in parser {
        let json = serde_json::to_value(entry).unwrap();
        value.map_json(&json);
        // if first {
        //     println!("{:#?}", json);
        //     println!("{:#?}", value);
        // }
        data.push(ParsedData {
            index_str: "".to_string(),
            data: json.to_string().as_bytes().to_vec(),
        });
        first = false;
    }
    println!("Parsed: {:#?} sec", now.elapsed().as_secs());
    let now = std::time::Instant::now();
    println!("Total records {:#?}", data.len());
    while !data.is_empty() {
        let mut batch = Vec::new();
        for _ in 0..10000 {
            if let Some(entry) = data.pop() {
                batch.push(entry);
            }
        }
        let res = client.insert(batch).unwrap();
        println!("{:#?}", "Send batch");
    }
    println!("Uploaded: {:#?} secs", now.elapsed().as_secs());
    println!("{:#?}", value);
}
