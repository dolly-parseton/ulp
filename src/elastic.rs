use crate::error::CustomError;
use crate::type_map::{IndexPatternObject, Mapping};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};
use type_casting::Types as TypeMap;

pub fn send_mapping(index: String, data: TypeMap) -> Result<(), CustomError> {
    let client = reqwest::blocking::Client::new();
    // loop {
    let res = client
        .put(format!(
            "http://0.0.0.0:9200/{}",
            sanitise_string_elastic(&index)
        ))
        .basic_auth("elastic", Some("changeme"))
        .header(reqwest::header::CONTENT_TYPE, "application/x-ndjson")
        .body(as_elastic_map(&data))
        .send()
        .map_err(|e| CustomError::ElasticError(e.into()))?;
    match res.status() {
        reqwest::StatusCode::OK => (),
        _ => {
            error!(
                "An error has occured whilst uploading an index mapping. {}",
                res.text()
                    .map_err(|e| CustomError::ElasticError(e.into()))?
            );
        }
    }
    // }
    Ok(())
}
//
fn as_elastic_map(map: &TypeMap) -> String {
    return format!("{{\"mappings\":{}}}", recurse(map));
    fn recurse(t: &TypeMap) -> String {
        use TypeMap::*;
        match t {
            Null => String::from("{\"type\": \"keyword\",\"null_value\": \"NULL\"}"),
            Bool => String::from("{\"type\": \"boolean\"}"),
            // UnsignedInteger => String::from("{\"type\": \"unsigned_long\"}"),
            Int => String::from("{\"type\": \"long\"}"),
            Float => String::from("{\"type\": \"double\"}"),
            IPv4 => String::from("{\"type\": \"ip\"}"),
            IPv6 => String::from("{\"type\": \"ip\"}"),
            Date => String::from("{\"type\": \"date\", \"format\": \"yyyy-MM-dd HH:mm:ss||yyyy-MM-dd||epoch_millis||date_optional_time||basic_ordinal_date_time\"}"),
            Str => String::from("{\"type\": \"text\", \"fields\": {\"keyword\": {\"type\": \"keyword\", \"ignore_above\": 256}}}"),
            List(_) => {
                String::from("{\"type\": \"text\", \"fields\": {\"keyword\": {\"type\": \"keyword\", \"ignore_above\": 256}}}")
            },
            Object(ref m) => {
                let mut parts: Vec<String> = Vec::new();
                for (k, v) in m {
                    parts.push(format!("\"{}\": {} ", k, recurse(v)));
                    // map.insert(k.to_string(), String::new(format!("{{\"properties\":{}}}", recurse(v.clone()))));
                }
                // let string = parts.join(",");
                format!("{{\"properties\": {{{}}}}}", parts.join(","))
            },
        }
    }
}

pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub fn normalise_then_send(
    map: Mapping,
    data: PathBuf,
    parser: &crate::Parser,
) -> Result<(), CustomError> {
    let mut buffer = Vec::with_capacity(1000);
    let index_pattern: IndexPatternObject = parser.default_index_pattern().into();
    for line in read_lines(data).map_err(|e| CustomError::ElasticError(e.into()))? {
        let json = serde_json::from_str(&line.map_err(|e| CustomError::ElasticError(e.into()))?)
            .map_err(|e| CustomError::ElasticError(e.into()))?;
        let data_pattern = index_pattern.generate_index_pattern(&json);
        let json = map.cast_json(json, Some(&data_pattern))?;
        buffer.push((data_pattern, json));
        if buffer.len() == buffer.capacity() {
            println!("{:?}", std::mem::size_of_val(&*buffer));
            bulk_api(&mut buffer).map_err(|e| CustomError::ElasticError(e))?;
        }
    }
    if !buffer.is_empty() {
        println!("{:?}", std::mem::size_of_val(&*buffer));
        bulk_api(&mut buffer).map_err(|e| CustomError::ElasticError(e))?;
    }
    Ok(())
}

pub fn bulk_api(
    buffer: &mut Vec<(String, serde_json::Value)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut req_body = String::new();
    let mut req_by_uuid = BTreeMap::new();
    for (pattern, json) in buffer.drain(..) {
        // Bulk API + Elastic Drain
        let uuid = uuid::Uuid::new_v4();
        let index_obj = format!(
            "{{\"index\" : {{ \"_index\" : \"{}\", \"_id\" : \"{}\"}} }}",
            sanitise_string_elastic(&pattern),
            uuid
        );
        //
        let json_str = serde_json::to_string(&json)?; // Think I can get away with as_str()
        req_body.push_str(&index_obj);
        req_body.push('\n');
        req_body.push_str(&json_str);
        req_body.push('\n');
        //
        req_by_uuid.insert(uuid, (pattern, json_str));
    }
    let client = reqwest::blocking::Client::new();
    loop {
        let res_raw = client
            .post("http://0.0.0.0:9200/_bulk?refresh=wait_for")
            .basic_auth("elastic", Some("changeme"))
            .header(reqwest::header::CONTENT_TYPE, "application/x-ndjson")
            .body(req_body.clone())
            .send()?;
        // .json()?;
        let text: serde_json::Value = res_raw.json()?;
        let res: response::BulkResponse = match serde_json::from_value(text.clone()) {
            Ok(r) => r,
            Err(e) => {
                println!("{:?}", e);
                println!("{:?}", text.to_string());
                // println!("{:?}", req_body.to_string());
                return Err(e.into());
            }
        };
        match res.contains_errors() {
            true => match res.has_bulk_rejection_errors() {
                Some(errors) => {
                    println!("Bulk rejection errors detected, retrying \n{:#?}", errors);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
                None => break,
            },
            false => break,
        }
    }
    Ok(())
}

pub fn sanitise_string_elastic(source: &str) -> String {
    let mut tmp = source.to_string();
    let string = tmp.as_mut_str();
    // Make lowercase
    string.make_ascii_lowercase();
    // Trim bad start chars
    // Remove bad chars
    tmp.replace(':', "")
        .replace('\"', "")
        .replace('*', "")
        .replace('+', "")
        .replace('/', "")
        .replace('\\', "")
        .replace('|', "")
        .replace('?', "")
        .replace('#', "")
        .replace('%', "")
        .replace(':', "")
        .replace('>', "")
        .replace('<', "")
        // Additionally remove any spaces
        .replace(' ', "_")
        .trim_start_matches('_')
        .trim_start_matches('.')
        .trim_start_matches('-')
        .to_string()
}

mod response {
    use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
    #[derive(Debug, Deserialize, Clone)]
    pub struct BulkResponse {
        pub took: u64,
        pub errors: bool,
        #[serde(skip_serializing)]
        pub items: Vec<BulkItem>,
    }
    impl BulkResponse {
        pub fn contains_errors(&self) -> bool {
            self.errors
        }
        pub fn errored_items(&self) -> Vec<BulkItem> {
            self.items
                .iter()
                .filter(|i| match i {
                    BulkItem::Index {
                        error,
                        index: _,
                        result: _,
                        status: _,
                    } => error.is_some(),
                })
                .cloned()
                .collect::<Vec<BulkItem>>()
        }
        pub fn has_bulk_rejection_errors(&self) -> Option<Vec<BulkItem>> {
            let errors = self
                .items
                .iter()
                .filter(|i| match i {
                    BulkItem::Index {
                        error,
                        index: _,
                        result: _,
                        status: _,
                    } => match error {
                        Some(e) => e.is_bulk_rejection(),
                        None => false,
                    },
                })
                .cloned()
                .collect::<Vec<BulkItem>>();
            match errors.is_empty() {
                true => None,
                false => Some(errors),
            }
        }
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub enum BulkItem {
        #[serde(rename = "index")]
        Index {
            #[serde(rename = "_index")]
            index: String,
            result: String,
            status: u64,
            error: Option<BulkError>,
        },
    }
    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct BulkError {
        pub r#type: String,
        pub reason: String,
        pub index_uuid: String,
        pub shard: u64,
        pub index: String,
    }
    impl BulkError {
        pub fn is_bulk_rejection(&self) -> bool {
            self.reason == "es_rejected_execution_exception"
        }
        pub fn _resource_already_exists_exception(&self) -> bool {
            self.reason == "resource_already_exists_exception"
        }
    }
    impl Serialize for BulkResponse {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            // 3 is the number of fields in the struct.
            let mut state = serializer.serialize_struct("BulkResponse", 4)?;
            state.serialize_field("took", &self.took)?;
            state.serialize_field("errors", &self.errors)?;
            state.serialize_field("items", &self.items.len())?;
            let errored = self.errored_items();
            state.serialize_field("error_items", &errored)?;
            state.end()
        }
    }
}
// mod mapping {
//     use type_casting::ElasticTypes as TypeMap;
//     // use std::{collections::BTreeMap, fmt};
//     //
//     //
//     // Replace with serialiser
//     pub fn as_elastic_map(map: &TypeMap) -> String {
//         return format!("{{\"mappings\":{}}}", recurse(map));
//         fn recurse(t: &TypeMap) -> String {
//             match t {
//                         TypeMap::Null => String::from("{\"type\": \"keyword\",\"null_value\": \"NULL\"}"),
//                         TypeMap::Bool => String::from("{\"type\": \"boolean\"}"),
//                         TypeMap::UnsignedInteger => String::from("{\"type\": \"unsigned_long\"}"),
//                         TypeMap::SignedInteger => String::from("{\"type\": \"long\"}"),
//                         TypeMap::Double => String::from("{\"type\": \"double\"}"),
//                         TypeMap::IPv4 => String::from("{\"type\": \"ip\"}"),
//                         TypeMap::IPv6 => String::from("{\"type\": \"ip\"}"),
//                         TypeMap::Date => String::from("{\"type\": \"date\", \"format\": \"yyyy-MM-dd HH:mm:ss||yyyy-MM-dd||epoch_millis||date_optional_time||basic_ordinal_date_time\"}"),
//                         TypeMap::String => String::from("{\"type\": \"text\", \"fields\": {\"keyword\": {\"type\": \"keyword\", \"ignore_above\": 256}}}"),
//                         TypeMap::Array(_) => {
//                             String::from("{\"type\": \"text\", \"fields\": {\"keyword\": {\"type\": \"keyword\", \"ignore_above\": 256}}}")
//                         },
//                         TypeMap::Object(ref m) => {
//                             let mut parts: Vec<String> = Vec::new();
//                             for (k, v) in m {
//                                 parts.push(format!("\"{}\": {} ", k, recurse(v)));
//                                 // map.insert(k.to_string(), String::new(format!("{{\"properties\":{}}}", recurse(v.clone()))));
//                             }
//                             // let string = parts.join(",");
//                             format!("{{\"properties\": {{{}}}}}", parts.join(","))
//                         },
//                     }
//         }
//     }
// }
