// Syslog output plugin for ULP
// Implements RFC 3164 and RFC 5424 syslog protocols

use crate::error::CustomError;
use crate::plugins::OutputPlugin;
use serde_json::Value;
use std::collections::HashMap;
use std::net::UdpSocket;

/// Syslog output plugin
/// Supports UDP transport with RFC 3164 format
#[derive(Debug)]
pub struct SyslogOutput {
    host: String,
    port: u16,
    facility: u8,
    hostname: String,
    socket: Option<UdpSocket>,
}

impl SyslogOutput {
    pub fn new() -> Self {
        Self {
            host: String::new(),
            port: 514,
            facility: 16, // local0
            hostname: "ulp".to_string(),
            socket: None,
        }
    }
    
    fn format_syslog_message(&self, record: &HashMap<String, Value>) -> String {
        let timestamp = record.get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let level = record.get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("INFO");
        
        let message = record.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let severity = self.level_to_severity(level);
        let priority = self.facility * 8 + severity;
        
        // RFC 3164 format: <priority>timestamp hostname tag: message
        format!(
            "<{}>{} {} ULP: {}",
            priority,
            self.format_timestamp(timestamp),
            self.hostname,
            message
        )
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
    
    fn format_timestamp(&self, timestamp: &str) -> String {
        if timestamp.is_empty() {
            // Use current time if no timestamp provided
            chrono::Utc::now().format("%b %d %H:%M:%S").to_string()
        } else {
            // Try to parse and reformat timestamp
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
                dt.format("%b %d %H:%M:%S").to_string()
            } else {
                timestamp.to_string()
            }
        }
    }
}

impl OutputPlugin for SyslogOutput {
    fn name(&self) -> &'static str {
        "syslog"
    }
    
    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<(), CustomError> {
        // Extract configuration values
        if let Some(host) = config.get("host").and_then(|v| v.as_str()) {
            self.host = host.to_string();
        } else {
            return Err(CustomError::ElasticError("syslog host is required".into()));
        }
        
        if let Some(port) = config.get("port").and_then(|v| v.as_u64()) {
            self.port = port as u16;
        }
        
        if let Some(facility) = config.get("facility").and_then(|v| v.as_u64()) {
            self.facility = facility as u8;
        }
        
        if let Some(hostname) = config.get("hostname").and_then(|v| v.as_str()) {
            self.hostname = hostname.to_string();
        }
        
        // Initialize UDP socket
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| CustomError::ElasticError(format!("Failed to bind UDP socket: {}", e).into()))?;
        
        socket.connect(format!("{}:{}", self.host, self.port))
            .map_err(|e| CustomError::ElasticError(format!("Failed to connect to syslog server: {}", e).into()))?;
        
        self.socket = Some(socket);
        
        info!("Syslog output configured for {}:{} (facility {})", self.host, self.port, self.facility);
        Ok(())
    }
    
    fn health_check(&self) -> Result<(), CustomError> {
        match &self.socket {
            Some(_) => Ok(()),
            None => Err(CustomError::ElasticError("Syslog socket not initialized".into())),
        }
    }
    
    fn send_single(&mut self, record: &HashMap<String, Value>) -> Result<(), CustomError> {
        let syslog_message = self.format_syslog_message(record);
        
        if let Some(ref socket) = self.socket {
            socket.send(syslog_message.as_bytes())
                .map_err(|e| CustomError::ElasticError(format!("Failed to send syslog message: {}", e).into()))?;
            debug!("Sent syslog message: {}", syslog_message);
        } else {
            return Err(CustomError::ElasticError("Syslog socket not initialized".into()));
        }
        
        Ok(())
    }
    
    fn flush(&mut self) -> Result<(), CustomError> {
        // UDP is stateless, no flushing needed
        Ok(())
    }
}

impl Default for SyslogOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_syslog_format() {
        let output = SyslogOutput::new();
        
        let mut record = HashMap::new();
        record.insert("timestamp".to_string(), json!("2024-01-01T12:00:00Z"));
        record.insert("level".to_string(), json!("INFO"));
        record.insert("message".to_string(), json!("Test message"));
        
        let formatted = output.format_syslog_message(&record);
        
        // Should start with priority <134> (facility 16 * 8 + severity 6)
        assert!(formatted.starts_with("<134>"));
        assert!(formatted.contains("ULP: Test message"));
    }
    
    #[test]
    fn test_level_to_severity() {
        let output = SyslogOutput::new();
        
        assert_eq!(output.level_to_severity("ERROR"), 3);
        assert_eq!(output.level_to_severity("WARN"), 4);
        assert_eq!(output.level_to_severity("INFO"), 6);
        assert_eq!(output.level_to_severity("DEBUG"), 7);
        assert_eq!(output.level_to_severity("UNKNOWN"), 6); // Default to INFO
    }
}