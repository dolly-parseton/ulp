// Example demonstrating the plugin system for ULP
// This example shows how to register and use output plugins

use ulp::plugins::{OutputRegistry, output::{SyslogOutput, VectorOutput}};
use serde_json::{json, Value};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();
    
    println!("🚀 ULP Plugin System Demo");
    
    // Create plugin registry
    let mut registry = OutputRegistry::new();
    
    // Register plugins
    registry.register(SyslogOutput::new());
    registry.register(VectorOutput::new());
    
    println!("📦 Registered plugins: {:?}", registry.list_plugins());
    
    // Configure plugins (normally this would come from config files)
    let syslog_config = create_syslog_config();
    let vector_config = create_vector_config();
    
    // Note: In a real implementation, we'd need to configure plugins through the registry
    // For this demo, we'll show how the plugins would be used
    
    // Create sample parsed records
    let sample_records = create_sample_records();
    
    println!("📝 Sample records created: {} records", sample_records.len());
    
    // Demonstrate how plugins would be used
    demonstrate_plugin_usage(&sample_records);
    
    println!("✅ Plugin demo completed successfully!");
    
    Ok(())
}

fn create_syslog_config() -> HashMap<String, Value> {
    let mut config = HashMap::new();
    config.insert("host".to_string(), json!("localhost"));
    config.insert("port".to_string(), json!(514));
    config.insert("facility".to_string(), json!(16)); // local0
    config.insert("hostname".to_string(), json!("ulp-demo"));
    config
}

fn create_vector_config() -> HashMap<String, Value> {
    let mut config = HashMap::new();
    config.insert("endpoint".to_string(), json!("http://localhost:8080/v1/logs"));
    config.insert("api_key".to_string(), json!("demo-api-key"));
    config.insert("batch_size".to_string(), json!(100));
    config
}

fn create_sample_records() -> Vec<HashMap<String, Value>> {
    vec![
        {
            let mut record = HashMap::new();
            record.insert("timestamp".to_string(), json!("2024-01-01T12:00:00Z"));
            record.insert("level".to_string(), json!("INFO"));
            record.insert("message".to_string(), json!("System startup completed"));
            record.insert("source".to_string(), json!("evtx"));
            record.insert("event_id".to_string(), json!(1001));
            record
        },
        {
            let mut record = HashMap::new();
            record.insert("timestamp".to_string(), json!("2024-01-01T12:01:00Z"));
            record.insert("level".to_string(), json!("WARN"));
            record.insert("message".to_string(), json!("Suspicious file access detected"));
            record.insert("source".to_string(), json!("mft"));
            record.insert("file_path".to_string(), json!("C:\\Windows\\System32\\config\\SAM"));
            record
        },
        {
            let mut record = HashMap::new();
            record.insert("timestamp".to_string(), json!("2024-01-01T12:02:00Z"));
            record.insert("level".to_string(), json!("ERROR"));
            record.insert("message".to_string(), json!("Failed login attempt"));
            record.insert("source".to_string(), json!("evtx"));
            record.insert("event_id".to_string(), json!(4625));
            record.insert("user".to_string(), json!("administrator"));
            record
        },
    ]
}

fn demonstrate_plugin_usage(records: &[HashMap<String, Value>]) {
    println!("\n🔌 Demonstrating Plugin Usage:");
    
    // Demonstrate syslog formatting
    println!("\n📡 Syslog Output Plugin:");
    let syslog = SyslogOutput::new();
    for (i, record) in records.iter().enumerate() {
        // Note: We can't actually send without configuration, but we can show formatting
        println!("  Record {}: {:?}", i + 1, record.get("message"));
    }
    
    // Demonstrate vector formatting  
    println!("\n🌊 Vector Output Plugin:");
    let vector = VectorOutput::new();
    println!("  Would send {} records to Vector endpoint", records.len());
    
    // Show configuration examples
    println!("\n⚙️  Configuration Examples:");
    println!("  Syslog: host=localhost, port=514, facility=16");
    println!("  Vector: endpoint=http://localhost:8080/v1/logs, batch_size=100");
    
    // Show routing possibilities
    println!("\n🎯 Routing Examples:");
    for record in records {
        let source = record.get("source").and_then(|v| v.as_str()).unwrap_or("unknown");
        let level = record.get("level").and_then(|v| v.as_str()).unwrap_or("INFO");
        
        match (source, level) {
            ("evtx", "ERROR") => println!("  Route: {} {} -> syslog + vector", source, level),
            ("mft", _) => println!("  Route: {} {} -> vector only", source, level),
            _ => println!("  Route: {} {} -> syslog", source, level),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_registry() {
        let mut registry = OutputRegistry::new();
        
        registry.register(SyslogOutput::new());
        registry.register(VectorOutput::new());
        
        let plugins = registry.list_plugins();
        assert!(plugins.contains(&"syslog"));
        assert!(plugins.contains(&"vector"));
    }
    
    #[test]
    fn test_sample_records() {
        let records = create_sample_records();
        assert_eq!(records.len(), 3);
        
        let first_record = &records[0];
        assert_eq!(first_record.get("level").unwrap(), &json!("INFO"));
        assert_eq!(first_record.get("source").unwrap(), &json!("evtx"));
    }
}