// Vector.dev output plugin for ULP
// Implements Vector's HTTP API for log ingestion

use crate::error::CustomError;
use crate::plugins::OutputPlugin;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Vector.dev output plugin
/// Sends logs to Vector via HTTP API with batching support
#[derive(Debug)]
pub struct VectorOutput {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
    batch_size: usize,
    buffer: Vec<HashMap<String, Value>>,
}

impl VectorOutput {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            endpoint: String::new(),
            api_key: None,
            batch_size: 1000,
            buffer: Vec::new(),
        }
    }
    
    fn format_vector_payload(&self, records: &[HashMap<String, Value>]) -> Result<String, CustomError> {
        let events: Vec<Value> = records
            .iter()
            .map(|record| {
                json!({
                    "timestamp": record.get("timestamp").unwrap_or(&json!(chrono::Utc::now().to_rfc3339())),
                    "message": record.get("message").unwrap_or(&json!("")),
                    "source": "ulp",
                    "level": record.get("level").unwrap_or(&json!("info")),
                    "metadata": record
                })
            })
            .collect();
        
        serde_json::to_string(&json!({ "events": events }))
            .map_err(|e| CustomError::ElasticError(format!("Failed to serialize Vector payload: {}", e).into()))
    }
    
    async fn send_to_vector(&self, payload: String) -> Result<(), CustomError> {
        let mut request = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .body(payload);
        
        if let Some(ref api_key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| CustomError::ElasticError(format!("Failed to send to Vector: {}", e).into()))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CustomError::ElasticError(
                format!("Vector returned status {}: {}", status, text).into()
            ));
        }
        
        Ok(())
    }
}

impl OutputPlugin for VectorOutput {
    fn name(&self) -> &'static str {
        "vector"
    }
    
    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<(), CustomError> {
        // Extract configuration values
        if let Some(endpoint) = config.get("endpoint").and_then(|v| v.as_str()) {
            self.endpoint = endpoint.to_string();
        } else {
            return Err(CustomError::ElasticError("vector endpoint is required".into()));
        }
        
        if let Some(api_key) = config.get("api_key").and_then(|v| v.as_str()) {
            self.api_key = Some(api_key.to_string());
        }
        
        if let Some(batch_size) = config.get("batch_size").and_then(|v| v.as_u64()) {
            self.batch_size = batch_size as usize;
        }
        
        info!("Vector output configured for endpoint: {}", self.endpoint);
        Ok(())
    }
    
    fn health_check(&self) -> Result<(), CustomError> {
        if self.endpoint.is_empty() {
            return Err(CustomError::ElasticError("Vector endpoint not configured".into()));
        }
        
        // TODO: Implement actual health check by sending a test request
        Ok(())
    }
    
    fn send_single(&mut self, record: &HashMap<String, Value>) -> Result<(), CustomError> {
        self.buffer.push(record.clone());
        
        if self.buffer.len() >= self.batch_size {
            self.flush()?;
        }
        
        Ok(())
    }
    
    fn send_batch(&mut self, records: &[HashMap<String, Value>]) -> Result<(), CustomError> {
        for record in records {
            self.buffer.push(record.clone());
        }
        
        if self.buffer.len() >= self.batch_size {
            self.flush()?;
        }
        
        Ok(())
    }
    
    fn flush(&mut self) -> Result<(), CustomError> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        
        let payload = self.format_vector_payload(&self.buffer)?;
        
        // Since this trait doesn't support async, we'll use a blocking call
        // In a real implementation, we might want to use a background task
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| CustomError::ElasticError(format!("Failed to create async runtime: {}", e).into()))?;
        
        rt.block_on(async {
            self.send_to_vector(payload).await
        })?;
        
        debug!("Sent {} records to Vector", self.buffer.len());
        self.buffer.clear();
        Ok(())
    }
}

impl Default for VectorOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_vector_payload_format() {
        let output = VectorOutput::new();
        
        let mut record = HashMap::new();
        record.insert("timestamp".to_string(), json!("2024-01-01T12:00:00Z"));
        record.insert("level".to_string(), json!("info"));
        record.insert("message".to_string(), json!("Test message"));
        
        let records = vec![record];
        let payload = output.format_vector_payload(&records).unwrap();
        
        let parsed: Value = serde_json::from_str(&payload).unwrap();
        assert!(parsed.get("events").is_some());
        assert_eq!(parsed["events"].as_array().unwrap().len(), 1);
        
        let event = &parsed["events"][0];
        assert_eq!(event["source"], "ulp");
        assert_eq!(event["message"], "Test message");
    }
    
    #[test]
    fn test_configuration() {
        let mut output = VectorOutput::new();
        
        let mut config = HashMap::new();
        config.insert("endpoint".to_string(), json!("http://localhost:8080/v1/logs"));
        config.insert("api_key".to_string(), json!("test-key"));
        config.insert("batch_size".to_string(), json!(500));
        
        output.configure(&config).unwrap();
        
        assert_eq!(output.endpoint, "http://localhost:8080/v1/logs");
        assert_eq!(output.api_key, Some("test-key".to_string()));
        assert_eq!(output.batch_size, 500);
    }
}