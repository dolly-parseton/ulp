# ULP Plugin Development Guide

## Overview

This guide describes how to develop plugins for ULP (Untitled Log Parser), including parser plugins, output plugins, and transform plugins.

## Parser Plugin Development

### Basic Parser Plugin Structure

```rust
use ulp::{ParserPlugin, ParsedOutput, ParserConfig, ParserError};
use std::path::Path;

pub struct MyCustomParser {
    name: String,
    version: String,
}

impl ParserPlugin for MyCustomParser {
    fn name(&self) -> &'static str {
        "my_custom_parser"
    }
    
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec![".mylog", ".custom"]
    }
    
    fn magic_bytes(&self) -> Vec<&'static [u8]> {
        vec![
            b"MYLOG",  // File signature
            &[0x4D, 0x59, 0x4C, 0x4F, 0x47], // Same as above in hex
        ]
    }
    
    fn parse(
        &self, 
        input: &Path, 
        config: &ParserConfig
    ) -> Result<ParsedOutput, ParserError> {
        // Implementation details
        self.parse_file(input, config)
    }
    
    fn schema(&self) -> JsonSchema {
        // Define the output schema
        JsonSchema::new()
            .add_field("timestamp", FieldType::DateTime)
            .add_field("level", FieldType::String)
            .add_field("message", FieldType::Text)
            .add_field("source", FieldType::String)
    }
    
    fn default_index_pattern(&self) -> &'static str {
        "mylog_{{source}}"
    }
}

impl MyCustomParser {
    pub fn new() -> Box<dyn ParserPlugin> {
        Box::new(Self {
            name: "MyCustomParser".to_string(),
            version: "1.0.0".to_string(),
        })
    }
    
    fn parse_file(
        &self, 
        input: &Path, 
        config: &ParserConfig
    ) -> Result<ParsedOutput, ParserError> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        
        let file = File::open(input)
            .map_err(|e| ParserError::IoError(e))?;
        let reader = BufReader::new(file);
        
        let mut records = Vec::new();
        
        for (line_num, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| ParserError::IoError(e))?;
            
            if let Some(record) = self.parse_line(&line, line_num)? {
                records.push(record);
            }
        }
        
        Ok(ParsedOutput {
            records,
            metadata: self.generate_metadata(input),
        })
    }
    
    fn parse_line(&self, line: &str, line_num: usize) -> Result<Option<ParsedRecord>, ParserError> {
        // Example: Parse a line like "2024-01-01 12:00:00 INFO [source] Message"
        let re = regex::Regex::new(r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) (\w+) \[([^\]]+)\] (.+)$")
            .map_err(|e| ParserError::RegexError(e))?;
        
        if let Some(captures) = re.captures(line) {
            let mut record = ParsedRecord::new();
            record.insert("timestamp", json!(captures.get(1).unwrap().as_str()));
            record.insert("level", json!(captures.get(2).unwrap().as_str()));
            record.insert("source", json!(captures.get(3).unwrap().as_str()));
            record.insert("message", json!(captures.get(4).unwrap().as_str()));
            record.insert("line_number", json!(line_num + 1));
            
            Ok(Some(record))
        } else {
            // Invalid line format - could log warning or skip
            Ok(None)
        }
    }
    
    fn generate_metadata(&self, input: &Path) -> HashMap<String, Value> {
        let mut metadata = HashMap::new();
        metadata.insert("parser".to_string(), json!("my_custom_parser"));
        metadata.insert("file_path".to_string(), json!(input.to_str().unwrap()));
        metadata.insert("parsed_at".to_string(), json!(chrono::Utc::now()));
        metadata
    }
}
```

### Advanced Parser Features

#### Streaming Parser for Large Files
```rust
impl MyCustomParser {
    fn parse_streaming(
        &self, 
        input: &Path, 
        config: &ParserConfig,
        callback: Box<dyn Fn(ParsedRecord) -> Result<(), ParserError>>
    ) -> Result<(), ParserError> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        
        let file = File::open(input)?;
        let reader = BufReader::new(file);
        
        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            
            if let Some(record) = self.parse_line(&line, line_num)? {
                callback(record)?;
            }
        }
        
        Ok(())
    }
}
```

#### Binary Format Parser
```rust
impl MyCustomParser {
    fn parse_binary(
        &self, 
        input: &Path, 
        config: &ParserConfig
    ) -> Result<ParsedOutput, ParserError> {
        use std::fs::File;
        use std::io::{Read, Seek, SeekFrom};
        use byteorder::{LittleEndian, ReadBytesExt};
        
        let mut file = File::open(input)?;
        let mut records = Vec::new();
        
        // Read header
        let magic = file.read_u32::<LittleEndian>()?;
        if magic != 0x474F4C59 { // "YLOG" in little endian
            return Err(ParserError::InvalidFormat("Invalid magic bytes".into()));
        }
        
        let version = file.read_u16::<LittleEndian>()?;
        let record_count = file.read_u32::<LittleEndian>()?;
        
        for _ in 0..record_count {
            let record = self.parse_binary_record(&mut file)?;
            records.push(record);
        }
        
        Ok(ParsedOutput {
            records,
            metadata: HashMap::new(),
        })
    }
    
    fn parse_binary_record(&self, file: &mut File) -> Result<ParsedRecord, ParserError> {
        use byteorder::{LittleEndian, ReadBytesExt};
        
        let timestamp = file.read_u64::<LittleEndian>()?;
        let level = file.read_u8()?;
        let message_len = file.read_u16::<LittleEndian>()? as usize;
        
        let mut message_bytes = vec![0u8; message_len];
        file.read_exact(&mut message_bytes)?;
        let message = String::from_utf8(message_bytes)
            .map_err(|e| ParserError::EncodingError(e))?;
        
        let mut record = ParsedRecord::new();
        record.insert("timestamp", json!(timestamp));
        record.insert("level", json!(level));
        record.insert("message", json!(message));
        
        Ok(record)
    }
}
```

### Parser Registration

#### Static Registration
```rust
// In your plugin initialization
pub fn register_parsers(registry: &mut ParserRegistry) {
    registry.register(MyCustomParser::new());
}
```

#### Dynamic Loading (Future)
```rust
// lib.rs for dynamic plugin
#[no_mangle]
pub extern "C" fn create_parser() -> *mut dyn ParserPlugin {
    Box::into_raw(MyCustomParser::new())
}

#[no_mangle]
pub extern "C" fn destroy_parser(parser: *mut dyn ParserPlugin) {
    unsafe {
        Box::from_raw(parser);
    }
}
```

## Output Plugin Development

### Basic Output Plugin Structure

```rust
use ulp::{OutputPlugin, OutputConfig, ParsedRecord, OutputError, ConfigError};

pub struct SyslogOutput {
    host: String,
    port: u16,
    facility: u8,
    transport: Transport,
    socket: Option<UdpSocket>,
}

#[derive(Debug, Clone)]
pub enum Transport {
    Udp,
    Tcp,
    Tls,
}

impl OutputPlugin for SyslogOutput {
    fn name(&self) -> &'static str {
        "syslog"
    }
    
    fn configure(&mut self, config: &OutputConfig) -> Result<(), ConfigError> {
        self.host = config.get_string("host")?;
        self.port = config.get_u16("port").unwrap_or(514);
        self.facility = config.get_u8("facility").unwrap_or(16);
        
        let transport_str = config.get_string("transport").unwrap_or("udp".to_string());
        self.transport = match transport_str.as_str() {
            "udp" => Transport::Udp,
            "tcp" => Transport::Tcp,
            "tls" => Transport::Tls,
            _ => return Err(ConfigError::InvalidValue("transport".into())),
        };
        
        self.initialize_connection()?;
        Ok(())
    }
    
    fn health_check(&self) -> Result<(), OutputError> {
        match &self.socket {
            Some(_) => Ok(()),
            None => Err(OutputError::NotConnected),
        }
    }
    
    fn send_single(&mut self, record: &ParsedRecord) -> Result<(), OutputError> {
        let syslog_message = self.format_syslog_message(record)?;
        self.send_message(&syslog_message)
    }
    
    fn send_batch(&mut self, records: &[ParsedRecord]) -> Result<(), OutputError> {
        for record in records {
            self.send_single(record)?;
        }
        Ok(())
    }
    
    fn flush(&mut self) -> Result<(), OutputError> {
        // For UDP, this is a no-op
        // For TCP/TLS, flush the stream
        Ok(())
    }
}

impl SyslogOutput {
    pub fn new() -> Box<dyn OutputPlugin> {
        Box::new(Self {
            host: String::new(),
            port: 514,
            facility: 16,
            transport: Transport::Udp,
            socket: None,
        })
    }
    
    fn initialize_connection(&mut self) -> Result<(), ConfigError> {
        match self.transport {
            Transport::Udp => {
                let socket = UdpSocket::bind("0.0.0.0:0")
                    .map_err(|e| ConfigError::ConnectionError(e.into()))?;
                socket.connect(format!("{}:{}", self.host, self.port))
                    .map_err(|e| ConfigError::ConnectionError(e.into()))?;
                self.socket = Some(socket);
            },
            Transport::Tcp => {
                // TCP implementation
                todo!("TCP transport not yet implemented");
            },
            Transport::Tls => {
                // TLS implementation
                todo!("TLS transport not yet implemented");
            }
        }
        Ok(())
    }
    
    fn format_syslog_message(&self, record: &ParsedRecord) -> Result<String, OutputError> {
        use chrono::{DateTime, Utc};
        
        let timestamp = record.get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .unwrap_or_else(|| Utc::now().into());
        
        let level = record.get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("INFO");
        
        let message = record.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let severity = self.level_to_severity(level);
        let priority = self.facility * 8 + severity;
        
        Ok(format!(
            "<{}>{} {} ULP: {}",
            priority,
            timestamp.format("%b %d %H:%M:%S"),
            "hostname", // Should be configurable
            message
        ))
    }
    
    fn level_to_severity(&self, level: &str) -> u8 {
        match level.to_uppercase().as_str() {
            "EMERGENCY" | "EMERG" => 0,
            "ALERT" => 1,
            "CRITICAL" | "CRIT" => 2,
            "ERROR" | "ERR" => 3,
            "WARNING" | "WARN" => 4,
            "NOTICE" => 5,
            "INFO" => 6,
            "DEBUG" => 7,
            _ => 6, // Default to INFO
        }
    }
    
    fn send_message(&mut self, message: &str) -> Result<(), OutputError> {
        if let Some(ref socket) = self.socket {
            socket.send(message.as_bytes())
                .map_err(|e| OutputError::SendError(e.into()))?;
        } else {
            return Err(OutputError::NotConnected);
        }
        Ok(())
    }
}
```

### Vector.dev Output Plugin

```rust
use reqwest::Client;
use serde_json::json;

pub struct VectorOutput {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
    batch_size: usize,
    compression: CompressionType,
    buffer: Vec<ParsedRecord>,
}

#[derive(Debug, Clone)]
pub enum CompressionType {
    None,
    Gzip,
    Lz4,
}

impl OutputPlugin for VectorOutput {
    fn name(&self) -> &'static str {
        "vector"
    }
    
    fn configure(&mut self, config: &OutputConfig) -> Result<(), ConfigError> {
        self.endpoint = config.get_string("endpoint")?;
        self.api_key = config.get_string("api_key").ok();
        self.batch_size = config.get_usize("batch_size").unwrap_or(1000);
        
        let compression_str = config.get_string("compression").unwrap_or("none".to_string());
        self.compression = match compression_str.as_str() {
            "none" => CompressionType::None,
            "gzip" => CompressionType::Gzip,
            "lz4" => CompressionType::Lz4,
            _ => return Err(ConfigError::InvalidValue("compression".into())),
        };
        
        Ok(())
    }
    
    fn send_single(&mut self, record: &ParsedRecord) -> Result<(), OutputError> {
        self.buffer.push(record.clone());
        
        if self.buffer.len() >= self.batch_size {
            self.flush()?;
        }
        
        Ok(())
    }
    
    fn send_batch(&mut self, records: &[ParsedRecord]) -> Result<(), OutputError> {
        for record in records {
            self.buffer.push(record.clone());
        }
        
        if self.buffer.len() >= self.batch_size {
            self.flush()?;
        }
        
        Ok(())
    }
    
    fn flush(&mut self) -> Result<(), OutputError> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        
        let payload = self.format_vector_payload(&self.buffer)?;
        let compressed_payload = self.compress_payload(payload)?;
        
        let mut request = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json");
        
        if let Some(ref api_key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        match self.compression {
            CompressionType::Gzip => {
                request = request.header("Content-Encoding", "gzip");
            },
            CompressionType::Lz4 => {
                request = request.header("Content-Encoding", "lz4");
            },
            _ => {}
        }
        
        let response = request
            .body(compressed_payload)
            .send()
            .map_err(|e| OutputError::SendError(e.into()))?;
        
        if !response.status().is_success() {
            return Err(OutputError::ServerError(format!(
                "Vector returned status: {}", 
                response.status()
            )));
        }
        
        self.buffer.clear();
        Ok(())
    }
    
    fn health_check(&self) -> Result<(), OutputError> {
        // Implement health check by sending a minimal request
        Ok(())
    }
}

impl VectorOutput {
    pub fn new() -> Box<dyn OutputPlugin> {
        Box::new(Self {
            client: Client::new(),
            endpoint: String::new(),
            api_key: None,
            batch_size: 1000,
            compression: CompressionType::None,
            buffer: Vec::new(),
        })
    }
    
    fn format_vector_payload(&self, records: &[ParsedRecord]) -> Result<String, OutputError> {
        let events: Vec<serde_json::Value> = records
            .iter()
            .map(|record| {
                json!({
                    "timestamp": record.get("timestamp"),
                    "message": record.to_string(),
                    "metadata": record
                })
            })
            .collect();
        
        serde_json::to_string(&json!({ "events": events }))
            .map_err(|e| OutputError::SerializationError(e.into()))
    }
    
    fn compress_payload(&self, payload: String) -> Result<Vec<u8>, OutputError> {
        match self.compression {
            CompressionType::None => Ok(payload.into_bytes()),
            CompressionType::Gzip => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;
                
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(payload.as_bytes())
                    .map_err(|e| OutputError::CompressionError(e.into()))?;
                encoder.finish()
                    .map_err(|e| OutputError::CompressionError(e.into()))
            },
            CompressionType::Lz4 => {
                // Implement LZ4 compression
                todo!("LZ4 compression not yet implemented");
            }
        }
    }
}
```

## Transform Plugin Development

### Basic Transform Plugin

```rust
use ulp::{TransformPlugin, ParsedRecord, TransformError};

pub struct FieldRenameTransform {
    mappings: HashMap<String, String>,
}

impl TransformPlugin for FieldRenameTransform {
    fn name(&self) -> &'static str {
        "field_rename"
    }
    
    fn transform(&self, mut input: ParsedRecord) -> Result<ParsedRecord, TransformError> {
        for (old_name, new_name) in &self.mappings {
            if let Some(value) = input.remove(old_name) {
                input.insert(new_name.clone(), value);
            }
        }
        Ok(input)
    }
    
    fn batch_transform(
        &self, 
        input: Vec<ParsedRecord>
    ) -> Result<Vec<ParsedRecord>, TransformError> {
        input.into_iter()
            .map(|record| self.transform(record))
            .collect()
    }
}

impl FieldRenameTransform {
    pub fn new(mappings: HashMap<String, String>) -> Box<dyn TransformPlugin> {
        Box::new(Self { mappings })
    }
}
```

### Complex Transform Example

```rust
pub struct EnrichmentTransform {
    geoip_db: Option<maxminddb::Reader<Vec<u8>>>,
    dns_cache: HashMap<std::net::IpAddr, String>,
}

impl TransformPlugin for EnrichmentTransform {
    fn name(&self) -> &'static str {
        "enrichment"
    }
    
    fn transform(&self, mut input: ParsedRecord) -> Result<ParsedRecord, TransformError> {
        // GeoIP enrichment
        if let Some(ip_str) = input.get("src_ip").and_then(|v| v.as_str()) {
            if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
                self.enrich_geoip(&mut input, ip)?;
                self.enrich_dns(&mut input, ip)?;
            }
        }
        
        // Add processing timestamp
        input.insert("processed_at".to_string(), json!(chrono::Utc::now()));
        
        Ok(input)
    }
}

impl EnrichmentTransform {
    fn enrich_geoip(
        &self, 
        record: &mut ParsedRecord, 
        ip: std::net::IpAddr
    ) -> Result<(), TransformError> {
        if let Some(ref reader) = self.geoip_db {
            match reader.lookup::<maxminddb::geoip2::City>(ip) {
                Ok(city) => {
                    if let Some(country) = city.country {
                        if let Some(name) = country.names {
                            if let Some(en_name) = name.get("en") {
                                record.insert("src_country".to_string(), json!(en_name));
                            }
                        }
                    }
                    
                    if let Some(city_data) = city.city {
                        if let Some(names) = city_data.names {
                            if let Some(en_name) = names.get("en") {
                                record.insert("src_city".to_string(), json!(en_name));
                            }
                        }
                    }
                }
                Err(_) => {
                    // GeoIP lookup failed - not necessarily an error
                }
            }
        }
        Ok(())
    }
    
    fn enrich_dns(
        &self, 
        record: &mut ParsedRecord, 
        ip: std::net::IpAddr
    ) -> Result<(), TransformError> {
        if let Some(hostname) = self.dns_cache.get(&ip) {
            record.insert("src_hostname".to_string(), json!(hostname));
        }
        Ok(())
    }
}
```

## Testing Plugins

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_custom_parser() {
        let parser = MyCustomParser::new();
        let config = ParserConfig::default();
        
        // Create test file
        let test_data = "2024-01-01 12:00:00 INFO [app] Test message\n";
        let temp_file = create_temp_file(test_data);
        
        let result = parser.parse(&temp_file, &config).unwrap();
        
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.records[0]["level"], json!("INFO"));
        assert_eq!(result.records[0]["message"], json!("Test message"));
    }
    
    #[test]
    fn test_syslog_output() {
        let mut output = SyslogOutput::new();
        
        let mut config = OutputConfig::new();
        config.set("host", "localhost");
        config.set("port", 514);
        
        output.configure(&config).unwrap();
        
        let mut record = ParsedRecord::new();
        record.insert("timestamp".to_string(), json!("2024-01-01T12:00:00Z"));
        record.insert("level".to_string(), json!("INFO"));
        record.insert("message".to_string(), json!("Test message"));
        
        // This would require a mock UDP socket for proper testing
        assert!(output.send_single(&record).is_ok());
    }
    
    fn create_temp_file(content: &str) -> PathBuf {
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.into_temp_path().to_path_buf()
    }
}
```

### Integration Testing

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_end_to_end_parsing() {
        let mut registry = ParserRegistry::new();
        registry.register(MyCustomParser::new());
        
        let mut output_manager = OutputManager::new();
        output_manager.register(SyslogOutput::new());
        
        // Create test job
        let job = Job::from_glob("/path/to/test/files/*.mylog").unwrap();
        
        // Process job
        let orchestrator = Orchestrator::new(registry, output_manager);
        let result = orchestrator.process_job(job).await;
        
        assert!(result.is_ok());
    }
}
```

## Plugin Packaging

### Cargo.toml Structure

```toml
[package]
name = "ulp-custom-parser"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
ulp = { path = "../ulp" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
regex = "1.0"
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
tempfile = "3.0"
tokio-test = "0.4"
```

### Plugin Manifest

```yaml
# plugin.yaml
name: "my_custom_parser"
version: "1.0.0"
description: "Custom log parser for MyLog format"
author: "Your Name <your.email@example.com>"
license: "MIT"

plugin_type: "parser"
supported_extensions: [".mylog", ".custom"]
magic_bytes: ["MYLOG"]

dependencies:
  ulp: "^0.1.0"

configuration:
  buffer_size:
    type: "integer"
    default: 8192
    description: "Read buffer size in bytes"
  
  strict_mode:
    type: "boolean"
    default: true
    description: "Fail on invalid lines"
```

## Best Practices

### Performance Considerations

1. **Use streaming parsers** for large files
2. **Implement batch processing** where possible
3. **Minimize memory allocations** in hot paths
4. **Use appropriate buffer sizes** for I/O operations
5. **Consider parallel processing** for independent records

### Error Handling

1. **Use specific error types** for different failure modes
2. **Provide context** in error messages
3. **Handle partial failures gracefully**
4. **Log appropriate diagnostic information**

### Configuration

1. **Provide sensible defaults** for all configuration options
2. **Validate configuration** during plugin initialization
3. **Document all configuration options**
4. **Support environment variable overrides**

### Testing

1. **Write comprehensive unit tests**
2. **Include integration tests** with real data
3. **Test error conditions** and edge cases
4. **Benchmark performance** with realistic datasets

This guide provides the foundation for developing robust, efficient plugins for the ULP system. The plugin architecture allows for easy extension while maintaining performance and reliability.