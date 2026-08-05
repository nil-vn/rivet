# Kiến trúc hệ thống

## 1. Architectural style

Rivet bắt đầu dưới dạng modular monolith nhưng module boundary được thiết kế như service boundary. Toàn bộ hệ thống biên dịch thành một executable; executable có thể chạy một hoặc nhiều vai trò:

```text
rivet node --role all
rivet node --role controller
rivet node --role executor
rivet serve
rivet run pipeline.toml
```

Trong single-node mode, các module trao đổi `RecordBatch` trực tiếp trong cùng process. Trong distributed mode, controller điều phối metadata, executor đọc/ghi dữ liệu trực tiếp, còn tabular exchange dùng Arrow Flight.

## 2. Nguyên tắc thiết kế

1. Arrow-native end-to-end.
2. Streaming mặc định; materialization phải tường minh.
3. Correctness trước throughput marketing.
4. Immutable input snapshot và immutable output artifact.
5. Event history là nguồn sự thật.
6. At-least-once execution, idempotent/transactional commit.
7. Bounded memory và end-to-end backpressure.
8. Controller không nằm trên data path.
9. Hai tầng DAG, không trộn orchestration với physical query scheduling.
10. Zero-copy nơi semantics cho phép, minimal-copy ở boundary bắt buộc.
11. Plugin capability và resource contract là một phần của scheduling.
12. Version pinning theo Arrow/DataFusion/Ballista BOM.

## 3. Sơ đồ tổng thể

```text
                         ┌──────────────────────────┐
                         │ BI / JDBC / ODBC clients │
                         └─────────────┬────────────┘
                                       │ Flight SQL
                         ┌─────────────▼────────────┐
                         │   Flight SQL Gateway     │
                         └─────────────┬────────────┘
                                       │
┌────────────────┐       ┌────────────▼─────────────┐       ┌─────────────────┐
│ Svelte / Apps  │◄─────►│ REST / WebSocket Gateway │       │ CLI / Cron/API  │
└────────────────┘ Arrow └────────────┬─────────────┘       └────────┬────────┘
                   IPC                │                              │
                         ┌────────────▼──────────────────────────────▼─┐
                         │               Control Plane                 │
                         │ DAG compiler, scheduler, history, leases    │
                         │ admission, registry, auth, reconciliation   │
                         └────────────┬────────────────────────────────┘
                                      │ Protobuf control messages
                    ┌─────────────────┼───────────────────┐
                    │                 │                   │
          ┌─────────▼────────┐ ┌──────▼──────────┐ ┌─────▼───────────┐
          │ Local Executor   │ │ Remote Executor │ │ Remote Executor │
          │ DataFusion       │ │ DataFusion      │ │ DataFusion      │
          │ Python / Wasm    │ │ Python / Wasm   │ │ Python / Wasm   │
          └─────────┬────────┘ └──────┬──────────┘ └─────┬───────────┘
                    │                 │                   │
                    └─────────┬───────┴───────────┬───────┘
                              │ local batch / Flight / durable artifact
                   ┌──────────▼───────────────────▼──────────┐
                   │          Unified Storage Layer          │
                   │ raw, Parquet, IPC, manifests, spill     │
                   │ local / S3 / R2 via object_store        │
                   └─────────────────────────────────────────┘
```

## 4. Component model

### 4.1 CLI và configuration

- Parse command/role/configuration.
- Load TOML/YAML-like declarative pipeline specification.
- Resolve secret references mà không ghi secret vào task history.
- Khởi tạo runtime profile: low, standard hoặc distributed-throughput.

### 4.2 Control plane

- Pipeline definition/version registry.
- DAG compiler và validator.
- Run/task state machine.
- Append-only event store và materialized state.
- Resource admission và tenant fairness.
- Executor registry, leases và heartbeats.
- Retry/cancellation/deadline.
- Artifact/manifest coordinator.
- Reconciliation và garbage collection.

### 4.3 Compute plane

- DataFusion `SessionContext`/`SessionState` factory.
- Logical/physical plan creation.
- Per-query target partitions và memory pool.
- Ballista adapter cho remote execution.
- Custom native transforms và bounded CPU pool.
- Spill directory và temporary disk accounting.

### 4.4 Data plane

- Local bounded `RecordBatch` transport.
- Flight `DoExchange`, `DoGet`, `DoPut` khi remote.
- Durable IPC/Parquet edge cho replay.
- Byte-credit flow control và cancellation propagation.

### 4.5 Discovery và ingestion

- Source snapshot/fingerprint.
- Encoding, dialect, header và schema discovery.
- Source segmentation.
- CSV partition planner và parser.
- Lossless bronze writer.
- Reject/quarantine writer.

### 4.6 Storage plane

- `object_store` registry.
- Local/S3/R2 implementations.
- Parquet writer/reader.
- Immutable artifact layout.
- Manifest generation và conditional commit.
- Metadata/statistics cache.

### 4.7 Plugin runtimes

- Rust-native registry.
- Trusted PyO3 runtime hoặc child worker.
- Sandboxed Wasmtime component runtime.
- Plugin version/capability/resource enforcement.

### 4.8 Serving plane

- Flight SQL.
- Axum REST/WebSocket.
- Authentication/authorization.
- Signed tickets và query lifecycle.
- Streaming output và slow-consumer backpressure.

## 5. Hai tầng DAG

### 5.1 Business DAG

```text
CSV Extract ──► Bronze Commit ──► Normalize SQL ──► Validate ──► Silver Commit
      └──────► Audit Statistics ────────────────────────────────────────┘
```

Business node có:

- `TaskId` ổn định.
- Retry/timeout/idempotency policy.
- Input/output artifact contract.
- Resource requirements.
- Capability placement requirements.
- History và checkpoint.

Petgraph chỉ được dùng để compile/traverse graph trong memory. Durable representation lưu `TaskId` và dependency pairs, không lưu `NodeIndex`.

### 5.2 Physical query DAG

Một business task SQL được DataFusion/Ballista hạ xuống:

```text
Scan partitions
      ├── filter/projection
      ├── hash/range repartition
      ├── local or partial aggregate
      └── final aggregate/sort
```

Stage boundary xuất hiện tại shuffle, repartition hoặc pipeline breaker. Business scheduler không được tự chia physical operator nếu DataFusion/Ballista đã làm việc đó.

### 5.3 Resource interaction

Business scheduler cấp một resource envelope cho query task. Query session sử dụng envelope đó để chọn `target_partitions`, memory pool và spill quota. Điều này ngăn nested oversubscription:

```text
Sai:
    8 concurrent DAG tasks × 16 DataFusion partitions trên máy 16 core

Đúng:
    admission controller chia 16 vcore giữa các task
    mỗi query chỉ tạo parallelism tương ứng allocation của nó
```

## 6. Execution modes

### 6.1 Single-node

- Controller, executor và serving cùng process.
- `RecordBatch` di chuyển qua bounded channel.
- DataFusion thực thi vectorized operators.
- Tokio xử lý async I/O, scheduler, timers và network.
- Custom CPU-bound work chạy trong fixed-size Rayon pool hoặc executor tương đương.
- Local Parquet và SQLite WAL là MVP persistence.

Tokio không phải CPU compute pool tổng quát. CPU loop dài trong async future sẽ chặn runtime worker tới `.await` tiếp theo.

### 6.2 Distributed

- Một hoặc nhiều controller instance tùy phase triển khai.
- Executor advertise vcore, RAM, temp disk, codecs và plugin capabilities.
- Controller lease partition/task; executor đọc source và ghi sink trực tiếp.
- SQL task dùng Ballista scheduler/executors khi phù hợp.
- Non-SQL task dùng application control gRPC, nhưng tabular payload vẫn dùng Flight/artifact.
- Shared storage hoặc executor Flight shuffle cung cấp intermediate access.

### 6.3 Standalone Ballista

Ballista standalone có thể được embed cho compatibility test, nhưng single-node production path nên giữ một DataFusion runtime trực tiếp để tránh scheduler overhead không cần thiết. Distributed feature được bật theo compile/runtime profile.

## 7. Data flow cho CSV → Parquet

```text
1. Resolve source URI
2. Create immutable SourceSnapshot
3. Sample encoding/dialect/header
4. Build safe source ranges
5. Lease ranges to executors
6. Decode and parse into RecordBatch stream
7. Detect inline schema/header drift
8. Write immutable bronze Parquet parts
9. Commit artifact and checkpoint
10. Merge artifact metadata
11. Publish dataset manifest generation
12. Trigger downstream tasks
```

Controller chỉ nhận metadata ở bước 5, 9, 10 và 11; không nhận bytes từ bước 6–8.

## 8. Transport abstraction

```rust
use std::pin::Pin;

use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use futures::Stream;

pub type BatchStream =
    Pin<Box<dyn Stream<Item = EngineResult<RecordBatch>> + Send>>;

#[derive(Debug, Clone)]
pub enum EdgeDurability {
    Ephemeral,
    Durable,
    Automatic,
}

#[derive(Debug, Clone)]
pub struct EdgeDescriptor {
    pub edge_id: String,
    pub durability: EdgeDurability,
    pub max_in_flight_bytes: u64,
    pub schema_fingerprint: String,
}

#[async_trait]
pub trait EdgeTransport: Send + Sync + 'static {
    async fn publish(
        &self,
        descriptor: EdgeDescriptor,
        input: BatchStream,
    ) -> EngineResult<ArtifactOrTicket>;

    async fn subscribe(
        &self,
        handle: ArtifactOrTicket,
    ) -> EngineResult<BatchStream>;
}
```

Implementation:

- `LocalEdgeTransport`: bounded channel và memory permit.
- `FlightEdgeTransport`: Flight ticket/DoExchange.
- `DurableEdgeTransport`: IPC/Parquet artifact trên object store.

`Automatic` được DAG compiler resolve thành ephemeral hoặc durable theo placement và recovery requirement.

`flume` có thể là implementation khởi đầu cho local MPMC channel, nhưng kiến trúc không phụ thuộc nhãn “lock-free”. Contract quan trọng là ownership transfer, bounded bytes, cancellation và backpressure; implementation channel phải được benchmark và có thể thay thế.

Transport này thay Kafka trong phạm vi pipeline edge, không phải một distributed event log tổng quát. Nếu use case cần retained topics, consumer groups độc lập, arbitrary replay window và external producers/consumers, đó là capability riêng chứ không được giả lập bằng ephemeral channel.

## 9. Zero-copy boundary matrix

| Boundary | Đánh giá | Lý do |
|---|---|---|
| Rust `RecordBatch` cùng process | Gần zero-copy | Move ownership hoặc clone `Arc` metadata |
| Projection/slice | Thường zero-copy | Dùng buffer view/offset |
| Filter/join/sort/aggregate | Có allocation | Kernel tạo output/index/state buffers |
| Shift-JIS/UTF-16 → UTF-8 | Bắt buộc copy | Byte representation thay đổi |
| Rust ↔ PyArrow cùng process | Zero-copy khả thi | Arrow C Data/C Stream |
| PyArrow ↔ Pandas | Không bảo đảm | Dtype/null/string conversion có thể copy |
| Rust ↔ Wasm linear memory | Thường copy | Guest memory tách khỏi host |
| Arrow Flight qua network | Minimal-copy | IPC framing, gRPC/TLS và kernel/network copies |
| mmap compressed Parquet | Không zero-copy decode | Pages vẫn phải decompress thành Arrow buffers |

Kiến trúc không được dùng “zero-copy” làm nhãn cho toàn pipeline. Metric cần đo bytes copied/allocation theo từng stage.

## 10. Core plugin contracts

### 10.1 Datasource

```rust
use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::datasource::TableProvider;
use url::Url;

#[derive(Debug, Clone)]
pub struct PluginDescriptor {
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SourceSpec {
    pub uri: Url,
    pub options: BTreeMap<String, String>,
    pub schema_hint: Option<arrow::datatypes::SchemaRef>,
}

#[derive(Clone)]
pub struct PluginContext {
    pub run_id: String,
    pub task_id: String,
    pub cancellation: tokio_util::sync::CancellationToken,
    pub object_stores: Arc<ObjectStoreRegistry>,
}

#[async_trait]
pub trait DataSourcePlugin: Send + Sync + 'static {
    fn descriptor(&self) -> &PluginDescriptor;

    fn accepts(&self, uri: &Url) -> bool;

    async fn discover(
        &self,
        context: PluginContext,
        spec: SourceSpec,
    ) -> EngineResult<DiscoveryManifest>;

    async fn create_provider(
        &self,
        context: PluginContext,
        spec: SourceSpec,
        manifest: DiscoveryManifest,
    ) -> EngineResult<Arc<dyn TableProvider>>;
}
```

Datasource trả `TableProvider` để DataFusion vẫn có projection/filter/limit pushdown và physical partition planning.

### 10.2 Lakehouse sink

```rust
use async_trait::async_trait;
use datafusion::physical_plan::SendableRecordBatchStream;

#[derive(Debug, Clone)]
pub struct SinkSpec {
    pub target_uri: String,
    pub partition_columns: Vec<String>,
    pub options: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PartitionWrite {
    pub partition_id: usize,
    pub object_uri: String,
    pub row_count: u64,
    pub byte_count: u64,
    pub content_hash: String,
}

#[derive(Debug, Clone)]
pub struct CommitReceipt {
    pub dataset_uri: String,
    pub manifest_uri: String,
    pub generation: String,
}

#[async_trait]
pub trait LakehouseSink: Send + Sync + 'static {
    fn descriptor(&self) -> &PluginDescriptor;

    async fn begin(
        &self,
        context: PluginContext,
        spec: SinkSpec,
        schema: arrow::datatypes::SchemaRef,
    ) -> EngineResult<Arc<dyn SinkTransaction>>;
}

#[async_trait]
pub trait SinkTransaction: Send + Sync + 'static {
    async fn write_partition(
        &self,
        partition_id: usize,
        input: SendableRecordBatchStream,
    ) -> EngineResult<PartitionWrite>;

    async fn commit(
        &self,
        partitions: Vec<PartitionWrite>,
    ) -> EngineResult<CommitReceipt>;

    async fn abort(&self) -> EngineResult<()>;
}
```

### 10.3 DAG scheduler

```rust
use std::collections::BTreeMap;

use async_trait::async_trait;
use petgraph::stable_graph::StableDiGraph;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub String);

#[derive(Debug, Clone)]
pub struct ResourceRequest {
    pub vcores: u16,
    pub memory_bytes: u64,
    pub temporary_disk_bytes: u64,
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TaskSpec {
    pub id: TaskId,
    pub kind: TaskKind,
    pub resources: ResourceRequest,
    pub max_attempts: u16,
    pub timeout_ms: u64,
    pub idempotency_key: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct DagDefinition {
    pub tasks: BTreeMap<TaskId, TaskSpec>,
    pub dependencies: Vec<Dependency>,
}

pub struct CompiledDag {
    pub graph: StableDiGraph<TaskId, EdgeDurability>,
    pub tasks: BTreeMap<TaskId, TaskSpec>,
}

#[async_trait]
pub trait DagScheduler: Send + Sync + 'static {
    async fn validate(
        &self,
        definition: DagDefinition,
    ) -> EngineResult<CompiledDag>;

    async fn submit(
        &self,
        definition: DagDefinition,
    ) -> EngineResult<String>;

    async fn cancel(&self, run_id: &str) -> EngineResult<()>;
}
```

## 11. State and consistency model

- Task execution: at-least-once.
- Source snapshot: immutable.
- Artifacts: immutable.
- Checkpoint: monotonic per partition/attempt contract.
- Manifest: generation-based, atomic publication.
- Side effects: sink-specific idempotency/transaction contract.
- Query results: streaming, không durable mặc định; durable result phải được yêu cầu tường minh.

“Exactly-once” chỉ được dùng ở phạm vi:

```text
Exactly-once visible dataset effect
```

Nó không tự động áp dụng cho email, HTTP API hoặc RDBMS không có idempotency key/transaction.

## 12. Ballista hay custom distributed engine

### Dùng Ballista khi

- Task là DataFusion logical/physical query.
- Có repartition, shuffle, distributed join/aggregate.
- Muốn dùng scheduler/executor và Flight shuffle sẵn có.

### Dùng application scheduler khi

- Task là source acquisition, REST call, Python/Wasm, sink commit hoặc maintenance.
- Cần history/retry semantics cấp pipeline.
- Cần resource/capability placement không thuộc query engine.

### Không nên làm

- Tự serialize DataFusion physical plan bằng format không versioned.
- Xây một shuffle protocol thứ hai nếu Ballista đáp ứng.
- Ép tất cả non-query task vào Ballista.

## 13. High availability evolution

### MVP

- Single controller.
- SQLite WAL event/state store.
- Durable artifacts và source snapshots.
- Controller restart/replay.

### Production distributed v1

- Single active controller với external PostgreSQL/etcd hoặc durable shared metadata.
- Executors lease task và reconnect sau controller restart.

### HA phase

- Active/passive leader election hoặc embedded Raft.
- Idempotent command/event application.
- Fencing token trên manifest commit.

HA consensus không được đưa vào MVP nếu chưa ổn định correctness của single-controller recovery.

## 14. Tài liệu tham khảo kỹ thuật

- [Apache DataFusion](https://datafusion.apache.org/user-guide/introduction.html)
- [Ballista architecture](https://datafusion.apache.org/ballista/contributors-guide/architecture.html)
- [Arrow C Data Interface](https://arrow.apache.org/docs/13.0/format/CDataInterface.html)
- [Arrow Flight specification](https://arrow.apache.org/docs/format/Flight.html)
- [Tokio CPU-bound guidance](https://docs.rs/tokio/latest/tokio/)
- [Petgraph topological sort](https://docs.rs/petgraph/latest/petgraph/algo/fn.toposort.html)
