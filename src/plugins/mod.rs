// Plugin system module for ULP
// This module provides traits and interfaces for extending ULP with new parsers and outputs

pub mod output;

use crate::error::CustomError;
use serde_json::Value;
use std::collections::HashMap;

/// Trait for implementing output plugins
pub trait OutputPlugin: Send + Sync {
    /// Return the name of this output plugin
    fn name(&self) -> &'static str;
    
    /// Configure the plugin with the given configuration
    fn configure(&mut self, config: &HashMap<String, Value>) -> Result<(), CustomError>;
    
    /// Check if the output is healthy and ready to receive data
    fn health_check(&self) -> Result<(), CustomError>;
    
    /// Send a single record to the output
    fn send_single(&mut self, record: &HashMap<String, Value>) -> Result<(), CustomError>;
    
    /// Send a batch of records to the output (default implementation calls send_single)
    fn send_batch(&mut self, records: &[HashMap<String, Value>]) -> Result<(), CustomError> {
        for record in records {
            self.send_single(record)?;
        }
        Ok(())
    }
    
    /// Flush any buffered data
    fn flush(&mut self) -> Result<(), CustomError>;
}

/// Registry for managing output plugins
pub struct OutputRegistry {
    plugins: HashMap<String, Box<dyn OutputPlugin>>,
}

impl OutputRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    
    /// Register a new output plugin
    pub fn register<P: OutputPlugin + 'static>(&mut self, plugin: P) {
        self.plugins.insert(plugin.name().to_string(), Box::new(plugin));
    }
    
    /// List all registered plugin names
    pub fn list_plugins(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }
    
    /// Check if a plugin exists
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
    
    /// Send data to a specific plugin
    pub fn send_to_plugin(&mut self, plugin_name: &str, records: &[HashMap<String, Value>]) -> Result<(), CustomError> {
        if let Some(plugin) = self.plugins.get_mut(plugin_name) {
            plugin.send_batch(records)
        } else {
            Err(CustomError::ElasticError(format!("Plugin '{}' not found", plugin_name).into()))
        }
    }
    
    /// Flush a specific plugin
    pub fn flush_plugin(&mut self, plugin_name: &str) -> Result<(), CustomError> {
        if let Some(plugin) = self.plugins.get_mut(plugin_name) {
            plugin.flush()
        } else {
            Err(CustomError::ElasticError(format!("Plugin '{}' not found", plugin_name).into()))
        }
    }
    
    /// Health check for a specific plugin
    pub fn health_check_plugin(&self, plugin_name: &str) -> Result<(), CustomError> {
        if let Some(plugin) = self.plugins.get(plugin_name) {
            plugin.health_check()
        } else {
            Err(CustomError::ElasticError(format!("Plugin '{}' not found", plugin_name).into()))
        }
    }
}

impl Default for OutputRegistry {
    fn default() -> Self {
        Self::new()
    }
}