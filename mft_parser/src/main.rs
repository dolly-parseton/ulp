#[macro_use]
extern crate serde;
extern crate serde_json;

mod parser;
mod value;

fn main() {
    println!("Reading MFT!");
    let mut value = value::TypeMap::default();
    let mut first = true;
    let mut parser = parser::get_parser("./.test_data/$MFT").unwrap();
    for record in parser.iter_entries().collect::<Vec<_>>() {
        // println!("{:?}", record);
        let entry = mft::csv::FlatMftEntryWithName::from_entry(&record.unwrap(), &mut parser);
        // println!("{:?}", entry.full_path);
        let mut json = serde_json::to_value(&entry).unwrap();
        // println!("{:?}", json);
        value.map_json(&json);
        if first {
            println!("{:#?}", value);
        }
        first = false;

        // let data1 = r#"
        // {
        //     "name": "John Doe",
        //     "age": {
        //         "name": true,
        //         "age": 4567453,
        //         "phones": [
        //             "+44 12345awfawf67",
        //             "+44 23awfawf45678"
        //         ]
        //     },
        //     "phones": [
        //         "+44 1234567",
        //         0
        //     ]
        // }"#;

        // // Parse the string of data into serde_json::Value.
        // let v1: serde_json::Value = serde_json::from_str(data1).unwrap();

        // let data2 = r#"
        // {
        //     "name": "John Doe",
        //     "age": {
        //         "name": "FAaLSE",
        //         "age": -1090,
        //         "phones": [
        //             "+44 12345awfawf67",
        //             "+44 23awfawf45678"
        //         ]
        //     },
        //     "phones": [
        //         "+44 1234567",
        //         0
        //     ]
        // }"#;

        // // Parse the string of data into serde_json::Value.
        // let v2: serde_json::Value = serde_json::from_str(data2).unwrap();

        // let mut value1 = value::TypeMap::default();
        // value1.map_json(&v1);
        // let mut value2 = value::TypeMap::default();
        // value2.map_json(&v2);
        // println!("{:#?}", value1);
        // println!("{:#?}", value2);

        // value1.map_json(&v2);
        // println!("{:#?}", value1);

        // std::thread::sleep(std::time::Duration::from_millis(1000));
    }
    println!("{:#?}", value);
}
