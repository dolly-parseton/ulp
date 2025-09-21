# ULP Architecture Documentation

## Overview

ULP (Untitled Log Parser) is a modular, high-performance forensic log parsing system built in Rust. This document describes the current architecture and proposed extensibility patterns.

## Current Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────────┐
│                          ULP Core                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │    API      │  │ Orchestrator│  │   Parser    │            │
│  │   Layer     │◄─┤   Worker    │◄─┤   Engine    │            │
│  │  (Warp)     │  │    Pool     │  │             │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
│         │                 │                │                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │    Job      │  │    Queue    │  │   Type      │            │
│  │  Manager    │  │  Manager    │  │  Mapping    │            │
│  │             │  │             │  │   System    │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                        Output Layer                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │ Elasticsearch│  │   Future    │  │   Future    │            │
│  │  Integration│  │   Syslog    │  │  Vector.dev │            │
│  │             │  │             │  │             │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
└─────────────────────────────────────────────────────────────────┘
```

### Module Structure

```
src/
├── lib.rs              # Core library exports and parser enum
├── main.rs             # Application entry point
├── api.rs              # REST API routes and handlers
├── workerpool.rs       # Job orchestration and worker management
├── job.rs              # Job and task definitions
├── elastic.rs          # Elasticsearch output implementation
├── type_map.rs         # Type mapping and casting system
├── evtx.rs            # Windows Event Log parser
├── mft.rs             # NTFS Master File Table parser
└── error.rs           # Error types and handling
```

## Data Flow

### Request Processing Flow

```
HTTP Request → API Handler → Message Queue → Orchestrator → Worker Pool
                                                   ↓
                                            Task Distribution
                                                   ↓
                                              Parser Engine
                                                   ↓
                                              Type Mapping
                                                   ↓
                                             Output System
```

### Detailed Flow Description

1. **HTTP Request**: Client submits parsing job via REST API
2. **API Handler**: Validates request and creates job message
3. **Message Queue**: Queues job for processing
4. **Orchestrator**: Manages job lifecycle and worker allocation
5. **Worker Pool**: Distributes tasks across available workers
6. **Parser Engine**: Processes forensic artifacts based on file type
7. **Type Mapping**: Normalizes and casts parsed data
8. **Output System**: Sends processed data to configured destinations

## Core Components Detail

### 1. Parser Engine

#### Current Implementation
```rust
pub enum Parser {
    Evtx,    // Windows Event Log parser
    Mft,     // NTFS Master File Table parser
    None,    // Default/unknown parser
}

impl Parser {
    pub fn run_parser(&self, task: &Task) {
        match self {
            Self::Mft => { ... },
            Self::Evtx => { ... },
            _ => panic!("No Parser for this file"),
        }
    }
}
```

#### File Type Detection
- Magic byte analysis for format identification
- Extension-based fallback detection
- Configurable detection rules

### 2. Job Management System

#### Job Structure
```rust
pub struct Job {
    pub id: Uuid,
    pub paths: Vec<PathBuf>,
    pub status: Status,
    pub mapping: Arc<Mutex<Mapping>>,
    pub sent: Arc<Mutex<Vec<(Uuid, PathBuf)>>>,
    pub processed: Vec<Task>,
    pub completed: Instant,
}
```

#### Task Distribution
- Jobs broken into individual file tasks
- UUID tracking for each task
- Thread-safe job state management

### 3. Type Mapping System

#### Type Casting
```rust
pub struct Mapping {
    pub types: Types,
    pub indicies: BTreeMap<String, Types>,
    pub stats: BTreeMap<String, ParsedFileStats>,
}
```

#### Index Pattern Support
```rust
pub struct IndexPatternObject {
    pub parts: Vec<(String, bool)>,
}
```

- Dynamic index pattern generation
- Field-based routing for different artifact types
- Type-safe data transformation

### 4. Worker Pool Architecture

#### Orchestrator
```rust
pub struct Orchestrator {
    pub pool: Arc<Mutex<WorkerPool>>,
    pub completed_queue: Queue<Job>,
    pub processing_queue: Queue<Job>,
    pub worker_queue: Queue<Job>,
    pub api_queue: Queue<ApiMessageType>,
    pub job_store: Store<Option<Job>>,
}
```

#### Concurrency Model
- Configurable worker count via `ULP_WORKERS_N` environment variable
- Thread-safe queue management
- Lock-free where possible

## Extensibility Patterns

### 1. Parser Plugin Architecture

#### Proposed Parser Trait
```rust
pub trait ParserPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn supported_extensions(&self) -> Vec<&'static str>;
    fn magic_bytes(&self) -> Vec<&'static [u8]>;
    
    fn parse(
        &self, 
        input: &Path, 
        config: &ParserConfig
    ) -> Result<ParsedOutput, ParserError>;
    
    fn schema(&self) -> JsonSchema;
    fn default_index_pattern(&self) -> &'static str;
}
```

#### Parser Registration
```rust
pub struct ParserRegistry {
    parsers: HashMap<String, Box<dyn ParserPlugin>>,
}

impl ParserRegistry {
    pub fn register<P: ParserPlugin + 'static>(&mut self, parser: P) {
        self.parsers.insert(parser.name().to_string(), Box::new(parser));
    }
    
    pub fn detect_parser(&self, path: &Path) -> Option<&dyn ParserPlugin> {
        // Magic byte detection logic
        // Extension-based fallback
    }
}
```

### 2. Output Plugin Architecture

#### Output Trait Design
```rust
pub trait OutputPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn configure(&mut self, config: &OutputConfig) -> Result<(), ConfigError>;
    fn health_check(&self) -> Result<(), OutputError>;
    
    fn send_single(&mut self, record: &ParsedRecord) -> Result<(), OutputError>;
    fn send_batch(&mut self, records: &[ParsedRecord]) -> Result<(), OutputError>;
    fn flush(&mut self) -> Result<(), OutputError>;
}
```

#### Output Manager
```rust
pub struct OutputManager {
    outputs: HashMap<String, Box<dyn OutputPlugin>>,
    routing_rules: Vec<RoutingRule>,
}

pub struct RoutingRule {
    pub filter: RecordFilter,
    pub outputs: Vec<String>,
}
```

### 3. Transform Plugin Architecture

#### Transform Trait
```rust
pub trait TransformPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn transform(&self, input: ParsedRecord) -> Result<ParsedRecord, TransformError>;
    fn batch_transform(&self, input: Vec<ParsedRecord>) -> Result<Vec<ParsedRecord>, TransformError>;
}
```

## Configuration System

### Configuration Hierarchy
```rust
pub struct Config {
    pub parsers: HashMap<String, ParserConfig>,
    pub outputs: HashMap<String, OutputConfig>,
    pub transforms: Vec<TransformConfig>,
    pub routing: RoutingConfig,
    pub performance: PerformanceConfig,
}
```

### YAML Configuration Example
```yaml
parsers:
  evtx:
    enabled: true
    batch_size: 1000
    extract_metadata: true
  
  mft:
    enabled: true
    include_deleted: false
  
  winreg:
    enabled: true
    hive_types: ["SYSTEM", "SOFTWARE", "SAM"]

outputs:
  elasticsearch:
    hosts: ["localhost:9200"]
    username: "elastic"
    password: "changeme"
    bulk_size: 5000
  
  syslog:
    host: "syslog.example.com"
    port: 514
    facility: 16
    transport: "udp"
  
  vector:
    endpoint: "http://vector:8080/v1/logs"
    api_key: "${VECTOR_API_KEY}"
    compression: "gzip"

routing:
  rules:
    - filter: { "parser": "evtx", "event_id": [4624, 4625] }
      outputs: ["elasticsearch", "syslog"]
    
    - filter: { "parser": "mft" }
      outputs: ["elasticsearch", "vector"]

performance:
  workers: 8
  memory_limit: "2GB"
  temp_dir: "/tmp/ulp"
```

## Security Considerations

### Data Protection
- Encryption at rest for temporary files
- Secure credential management
- Audit logging for all operations

### Access Control
- API key authentication
- Role-based access control
- Rate limiting for API endpoints

### Input Validation
- File type verification
- Size limits for uploads
- Malformed data handling

## Performance Optimizations

### Memory Management
- Streaming parsers for large files
- Memory-mapped file access where beneficial
- Garbage collection optimization

### I/O Optimization
- Asynchronous file operations
- Batch processing for output operations
- Compression for network transfers

### Concurrency
- Lock-free data structures where possible
- Work-stealing job queues
- NUMA-aware thread allocation

## Monitoring and Observability

### Metrics Collection
```rust
pub struct Metrics {
    pub jobs_processed: Counter,
    pub records_parsed: Counter,
    pub parse_duration: Histogram,
    pub output_latency: Histogram,
    pub error_count: Counter,
}
```

### Health Checks
- Parser availability
- Output system connectivity
- Resource utilization monitoring
- Queue depth tracking

### Logging Strategy
- Structured logging with correlation IDs
- Configurable log levels
- Integration with log aggregation systems

## Deployment Architecture

### Standalone Deployment
```
┌─────────────────┐
│      ULP        │
│   Application   │
├─────────────────┤
│  Local Storage  │
└─────────────────┘
```

### Distributed Deployment
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   ULP Worker    │    │   ULP Worker    │    │   ULP Worker    │
│      Node       │    │      Node       │    │      Node       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │  Load Balancer  │
                    │   / API Gateway │
                    └─────────────────┘
```

### Container Architecture
```yaml
# docker-compose.yml
version: '3.8'
services:
  ulp:
    image: ulp:latest
    environment:
      - ULP_WORKERS_N=8
      - ELASTICSEARCH_URL=http://elasticsearch:9200
    volumes:
      - ./data:/data:ro
      - ./config:/config:ro
    depends_on:
      - elasticsearch
      - vector
```

## Future Architecture Considerations

### Microservices Architecture
- Separate services for parsing, transformation, and output
- Message queue-based communication
- Independent scaling and deployment

### Stream Processing
- Real-time log processing capabilities
- Apache Kafka integration
- Event-driven architecture

### Cloud-Native Features
- Kubernetes-native deployment
- Auto-scaling based on queue depth
- Cloud storage integration

This architecture provides a solid foundation for extending ULP while maintaining performance, reliability, and ease of use.