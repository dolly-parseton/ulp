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
            // Bulk API + Elastic Drain
            println!("Bulk API");
            bulk_api(&mut buffer).map_err(|e| CustomError::ElasticError(e.into()))?;
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
        req_body.push_str("\n");
        req_body.push_str(&json_str);
        req_body.push_str("\n");
    }
    println!("Sending data");
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("http://0.0.0.0:9200/_bulk?refresh=wait_for")
        .basic_auth("elastic", Some("changeme"))
        .header(reqwest::header::CONTENT_TYPE, "application/x-ndjson")
        .body(req_body)
        .send()?;
    println!("{:#?}", res.text());
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
