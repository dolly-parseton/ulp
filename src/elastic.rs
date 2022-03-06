use crate::{
    error::CustomError,
    type_map::{IndexPatternObject, Mapping},
};
use std::{
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub fn normalise_then_send(map: Mapping, data: PathBuf) -> Result<(), CustomError> {
    let mut buffer = Vec::with_capacity(1000);
    let index_pattern: IndexPatternObject = "evtx_{{Event.System.Provider_attributes.Name}}".into();
    for line in read_lines(data).map_err(|e| CustomError::ElasticError(e.into()))? {
        let mut json =
            serde_json::from_str(&line.map_err(|e| CustomError::ElasticError(e.into()))?)
                .map_err(|e| CustomError::ElasticError(e.into()))?;
        let data_pattern = index_pattern.generate_index_pattern(&json);
        map.cast_json(&mut json, None)?;
        buffer.push((data_pattern, json));
        if buffer.len() == buffer.capacity() {
            bulk_api(&mut buffer).map_err(|e| CustomError::ElasticError(e))?;
        }
    }
    Ok(())
}

pub fn bulk_api(
    buffer: &mut Vec<(String, serde_json::Value)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut req_body = String::new();
    for (pattern, json) in buffer.drain(..) {
        // Bulk API + Elastic Drain
        let index_obj = format!(
            "{{\"index\" : {{ \"_index\" : \"{}\"}} }}",
            sanitise_string_elastic(&pattern)
        );
        let json_str = serde_json::to_string(&json)?; // Think I can get away with as_str()
        req_body.push_str(&index_obj);
        req_body.push('\n');
        req_body.push_str(&json_str);
        req_body.push('\n');
    }
    let client = reqwest::blocking::Client::new();
    let res: response::BulkResponse = client
        .post("http://0.0.0.0:9200/_bulk?refresh=wait_for")
        .basic_auth("elastic", Some("changeme"))
        .header(reqwest::header::CONTENT_TYPE, "application/x-ndjson")
        .body(req_body)
        .send()?
        .json()?;
    println!("{:#?}", res);
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
            let errored = self
            .items
            .iter()
            .filter(|i| match i {
                BulkItem::Index {
                    error,
                    index: _,
                    result: _,
                    status: _,
                } => error.is_some(),
                // _ => false,
            })
            .cloned().collect::<Vec<BulkItem>>();
            state.serialize_field(
                "error_items",
                &errored,
            )?;
            state.end()
        }
    }
}

mod mapping {
    use crate::type_map::{ TypeMap};
    use std::{collections::BTreeMap, fmt};
    //
    pub struct ElasticIndex(TypeMap);
    //
    impl fmt::Display for ElasticIndex {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            match self.0 {
                TypeMap::Null => fmt.write_str("{\"type\": \"keyword\",\"null_value\": \"NULL\"}")?,
                TypeMap::Boolean => fmt.write_str("{\"type\": \"boolean\"}")?,
                TypeMap::UnsignedInteger => fmt.write_str("{\"type\": \"unsigned_long\"}")?,
                TypeMap::SignedInteger => fmt.write_str("{\"type\": \"long\"}")?,
                TypeMap::Double => fmt.write_str("{\"type\": \"double\"}")?,
                TypeMap::IPv4 => fmt.write_str("{\"type\": \"ip\"}")?,
                TypeMap::IPv6 => fmt.write_str("{\"type\": \"ip\"}")?,
                TypeMap::Date => fmt.write_str("{\"type\": \"date\", \"format\": \"yyyy-MM-dd HH:mm:ss||yyyy-MM-dd||epoch_millis||date_optional_time||basic_ordinal_date_time\"}")?,
                TypeMap::String => fmt.write_str("{\"type\": \"text\", \"fields\": {\"keyword\": {\"type\": \"keyword\", \"ignore_above\": 256}}}")?, 
                TypeMap::Array(_) => {
                    fmt.write_str("{\"type\": \"text\", \"fields\": {\"keyword\": {\"type\": \"keyword\", \"ignore_above\": 256}}}")?
                }, 
                TypeMap::Object(ref m) => {
                    let mut map: BTreeMap<String, Self> = BTreeMap::new();
                    for (k, v) in m {
                        map.insert(k.to_string(), Self(v.clone()));
                    }
                    fmt.write_fmt(format_args!("{{\"properties\":{:?}}}", map))?
                },
            };
            Ok(())
        }
    }
    impl fmt::Debug for ElasticIndex {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            match self.0 {
                TypeMap::Null => fmt.write_str("{\"type\": \"keyword\",\"null_value\": \"NULL\"}")?,
                TypeMap::Boolean => fmt.write_str("{\"type\": \"boolean\"}")?,
                TypeMap::UnsignedInteger => fmt.write_str("{\"type\": \"unsigned_long\"}")?,
                TypeMap::SignedInteger => fmt.write_str("{\"type\": \"long\"}")?,
                TypeMap::Double => fmt.write_str("{\"type\": \"double\"}")?,
                TypeMap::IPv4 => fmt.write_str("{\"type\": \"ip\"}")?,
                TypeMap::IPv6 => fmt.write_str("{\"type\": \"ip\"}")?,
                TypeMap::Date => fmt.write_str("{\"type\": \"date\", \"format\": \"yyyy-MM-dd HH:mm:ss||yyyy-MM-dd||epoch_millis||date_optional_time||basic_ordinal_date_time\"}")?,
                TypeMap::String => fmt.write_str("{\"type\": \"text\", \"fields\": {\"keyword\": {\"type\": \"keyword\", \"ignore_above\": 256}}}")?,
                TypeMap::Array(_) => {
                    fmt.write_str("{\"type\": \"text\", \"fields\": {\"keyword\": {\"type\": \"keyword\", \"ignore_above\": 256}}}")?
                }, 
                TypeMap::Object(ref m) => {
                    let mut map: BTreeMap<String, Self> = BTreeMap::new();
                    for (k, v) in m {
                        map.insert(k.to_string(),  Self(v.clone()));
                    }
                    fmt.write_fmt(format_args!("{{\"properties\":{:?}}}", map))?
                },
            };
            Ok(())
        }
    }
}
