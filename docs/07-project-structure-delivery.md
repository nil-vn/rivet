# Cấu trúc dự án, dependency, lộ trình và verification

## 1. Repository strategy

MVP dùng modular monolith để:

- Giữ một Arrow/DataFusion type universe.
- Giảm compile/dependency/version coordination.
- Dễ refactor khi domain boundary chưa ổn định.
- Tạo một executable đúng mục tiêu.

Chỉ tách Cargo workspace crates khi:

- Compile time hoặc ownership team yêu cầu.
- Module API đã ổn định.
- Cần sandbox/process boundary thực sự.

## 2. Cấu trúc thư mục đề xuất

```text
furrumx/
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── rust-toolchain.toml
├── rustfmt.toml
├── LICENSE
├── DCO
├── README.md
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── error.rs
│   ├── runtime.rs
│   ├── core/
│   │   ├── mod.rs
│   │   ├── ids.rs
│   │   ├── batch.rs
│   │   ├── artifact.rs
│   │   ├── capability.rs
│   │   ├── cancellation.rs
│   │   └── resource.rs
│   ├── dag/
│   │   ├── mod.rs
│   │   ├── model.rs
│   │   ├── compiler.rs
│   │   ├── scheduler.rs
│   │   ├── admission.rs
│   │   ├── lease.rs
│   │   ├── retry.rs
│   │   └── placement.rs
│   ├── history/
│   │   ├── mod.rs
│   │   ├── event.rs
│   │   ├── event_store.rs
│   │   ├── materializer.rs
│   │   ├── lineage.rs
│   │   └── reconciliation.rs
│   ├── checkpoint/
│   │   ├── mod.rs
│   │   ├── contract.rs
│   │   ├── store.rs
│   │   ├── recovery.rs
│   │   └── watermark.rs
│   ├── compute/
│   │   ├── mod.rs
│   │   ├── datafusion.rs
│   │   ├── ballista.rs
│   │   ├── partitioner.rs
│   │   ├── memory.rs
│   │   ├── spill.rs
│   │   └── metrics.rs
│   ├── discovery/
│   │   ├── mod.rs
│   │   ├── encoding.rs
│   │   ├── dialect.rs
│   │   ├── header.rs
│   │   ├── schema.rs
│   │   └── segment.rs
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── local.rs
│   │   ├── flight.rs
│   │   ├── durable.rs
│   │   ├── credit.rs
│   │   └── ticket.rs
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── registry.rs
│   │   ├── raw_zone.rs
│   │   ├── bronze.rs
│   │   ├── parquet.rs
│   │   ├── manifest.rs
│   │   ├── commit.rs
│   │   └── mmap.rs
│   ├── plugins/
│   │   ├── mod.rs
│   │   ├── registry.rs
│   │   ├── datasource.rs
│   │   ├── sink.rs
│   │   ├── csv/
│   │   │   ├── mod.rs
│   │   │   ├── decoder.rs
│   │   │   ├── scanner.rs
│   │   │   ├── parser.rs
│   │   │   ├── partition.rs
│   │   │   └── provider.rs
│   │   ├── python/
│   │   │   ├── mod.rs
│   │   │   ├── bridge.rs
│   │   │   └── worker.rs
│   │   └── wasm/
│   │       ├── mod.rs
│   │       ├── host.rs
│   │       ├── limits.rs
│   │       └── cache.rs
│   ├── serving/
│   │   ├── mod.rs
│   │   ├── flight_sql.rs
│   │   ├── rest.rs
│   │   ├── websocket.rs
│   │   ├── auth.rs
│   │   └── middleware.rs
│   └── control/
│       ├── mod.rs
│       ├── controller.rs
│       ├── executor.rs
│       ├── heartbeat.rs
│       └── proto.rs
├── proto/
│   └── control.proto
├── wit/
│   └── analytics.wit
├── migrations/
│   └── 0001_initial.sql
├── docs/
├── scripts/
│   ├── check-docs.sh
│   └── check-wsl.sh
├── examples/
│   ├── csv_to_parquet.toml
│   └── distributed_etl.toml
├── tests/
│   ├── integration/
│   ├── fault_injection/
│   ├── compatibility/
│   └── fixtures/
└── benches/
    ├── csv_ingest.rs
    ├── encoding.rs
    ├── parquet_write.rs
    ├── channel.rs
    ├── flight_exchange.rs
    └── recovery.rs
```

## 3. Dependency baseline

Baseline ngày 2026-08-05:

- Ballista 53 phụ thuộc DataFusion 53 và Arrow Flight 58.
- Không dùng DataFusion 54 trong distributed build cho tới khi Ballista tương ứng được nâng.
- Lockfile hiện resolve Arrow/Parquet/Flight 58.4.x và DataFusion 53.1.x.
- `object_store` resolve 0.13.2 để tương thích DataFusion/Ballista, dù upstream có dòng mới hơn.
- Arrow PyArrow 58.4 dùng PyO3-compatible 0.28 line; không ép PyO3 0.29 nếu tạo `links` conflict.
- Wasmtime 47 đặt package MSRV ở Rust 1.94; development/CI pin 1.97.1.

`Cargo.lock` phải commit và release build dùng `--locked`. Chi tiết và compile evidence nằm trong [ADR-0001](decisions/0001-dependency-bom-msrv.md).

## 4. Cargo.toml khởi điểm

Root `Cargo.toml` là source of truth. Scaffold private/WSL hiện dùng profile tối thiểu sau; dependency mới chỉ được thêm theo work package thực tế:

```toml
[package]
name = "furrumx"
version = "0.1.0"
edition = "2024"
rust-version = "1.94"
license = "MIT"
publish = false

[features]
default = ["local"]
local = ["dep:arrow", "dep:datafusion", "dep:object_store", "dep:parquet"]
flight-sql = ["local", "dep:arrow-flight", "arrow-flight/flight-sql"]
distributed = ["flight-sql", "dep:ballista", "dep:prost", "dep:tonic"]
wasm = ["dep:wasmtime", "dep:wasmtime-wasi"]
python = ["local", "arrow/pyarrow", "dep:pyo3"]
s3 = ["local", "object_store/aws"]
http-store = ["local", "object_store/http"]

[dependencies]
clap = { version = "4", features = ["derive", "env"] }
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

arrow = { version = "58.3", optional = true }
arrow-flight = { version = "58.3", optional = true }
parquet = { version = "58.3", optional = true }
datafusion = { version = "53.1", optional = true }
ballista = { version = "53.0", optional = true }
object_store = { version = "0.13.1", optional = true }
pyo3 = { version = "0.28.3", optional = true }
wasmtime = { version = "47.0", optional = true }
```

Wasmtime 47 đặt MSRV thực tế của full feature universe ở Rust 1.94. Development/CI pin Rust 1.97.1. Không đặt `panic = "abort"` trong baseline vì plugin/native panic isolation và graceful cleanup cần được đánh giá trước khi đổi.

## 5. Build profiles

### Minimal local

```text
local CSV
DataFusion
local Parquet
SQLite history
CLI
```

### Full local

```text
minimal
Flight SQL
REST/WebSocket
Wasm
```

### Distributed

```text
full local
Ballista
control gRPC
S3/R2
controller/executor roles
```

### Python

Build riêng có PyO3 và documented interpreter/package requirements.

CI phải compile/test feature matrix, tránh feature chỉ compile trong developer machine.

## 6. Pipeline specification

Pipeline definition phải declarative và versionable. Ví dụ:

```toml
[pipeline]
id = "customers_ingestion"
version = "1.0.0"

[[tasks]]
id = "discover_and_ingest"
kind = "source"
plugin = "csv"
vcores = 2
memory = "2GiB"
temporary_disk = "8GiB"
max_attempts = 4
timeout = "6h"

[tasks.source]
uri = "file:///data/incoming/customers.csv"

[tasks.source.snapshot]
mode = "copy_if_mutable"

[tasks.source.encoding]
mode = "detect"
candidates = ["utf-8", "utf-16le", "utf-16be", "shift_jis"]
ambiguity = "best_effort"

[tasks.source.csv]
delimiter_candidates = [",", "\t", ";", "|"]
max_header_rows = 5
repeated_header = "segment_or_skip"
schema_drift = "new_segment"

[tasks.output]
kind = "parquet"
uri = "file:///lakehouse/bronze/customers"
codec = "snappy"

[[tasks]]
id = "normalize"
kind = "sql"
sql_file = "sql/customers_normalize.sql"
vcores = 4
memory = "4GiB"

[[tasks]]
id = "publish_silver"
kind = "sink"
plugin = "parquet"

[[dependencies]]
upstream = "discover_and_ingest"
downstream = "normalize"
durability = "durable"

[[dependencies]]
upstream = "normalize"
downstream = "publish_silver"
durability = "durable"
```

Spec hash bao gồm normalized configuration nhưng secret plaintext bị loại và thay bằng secret version/reference identity phù hợp.

## 7. API/versioning rules

- Pipeline definition có explicit version.
- Protobuf field numbers không reuse.
- Event payload có version.
- Plugin API có `api_version`.
- Manifest có format version.
- Schema/header/dialect decision có fingerprint/version.
- Breaking storage format cần migration/reader compatibility plan.
- Resume contract thay đổi là breaking correctness change và phải invalidate checkpoint cũ.

## 8. ADR backlog

ADR nên được tạo khi bắt đầu implementation:

1. Arrow/DataFusion/Ballista BOM policy.
2. Business DAG và physical DAG separation.
3. Event store và materialized state.
4. Checkpoint-after-artifact invariant.
5. Raw/bronze/silver zone semantics.
6. Manifest format: Arrow IPC hay Protobuf.
7. Local commit/fsync guarantees theo OS/filesystem.
8. Resource/memory permit model.
9. Python trust/isolation modes.
10. Wasm WIT batch handle contract.
11. HA metadata backend.
12. Dataset catalog/Iceberg adoption.

## 9. Lộ trình triển khai

Phần này mô tả roadmap kiến trúc ở mức phase. Work packages, dependency graph, effort, release gates và iteration sequencing chi tiết nằm trong [kế hoạch phát triển](08-development-plan.md).

### Phase 0 — technical spikes

- Compile coherent dependency graph.
- Stream DataFusion `RecordBatch` vào Parquet.
- Prototype local byte-accounted channel.
- PyArrow C Stream round trip.
- Wasmtime component limits.
- Ballista remote query smoke test.
- Benchmark UTF-8/Shift-JIS/UTF-16 decode + CSV parse.

Exit criteria:

- Quyết định BOM và MSRV.
- Không có duplicate Arrow major versions trong intended feature profile.
- Có baseline throughput/memory numbers.

### Phase 1 — local resilient ingest MVP

- CLI/config.
- SQLite migrations/event store.
- Pipeline definition/DAG validation.
- Local scheduler/lease abstraction.
- Raw source snapshot.
- CSV encoding/dialect/header discovery.
- Streaming bronze batches.
- Local Parquet sink.
- Artifact/checkpoint/manifest protocol.
- Reconciler.
- Metrics và low-memory profile.

Exit criteria:

- Crash matrix tests pass.
- 1B-row synthetic ingest hoàn tất trong bounded memory.
- UTF-8/UTF-16/Shift-JIS và header drift fixtures pass.

### Phase 2 — SQL and serving

- DataFusion SQL tasks.
- Per-query admission/memory/spill.
- Flight SQL read-only.
- REST/WS Arrow stream.
- Authentication, signed tickets, cancellation.

Exit criteria:

- Slow-client backpressure test.
- BI driver compatibility matrix ban đầu.
- Concurrent ETL/interactive resource isolation.

### Phase 3 — extensibility/cloud

- Wasm runtime/manifest/capabilities.
- Trusted Python build profile.
- S3/R2 object store.
- Conditional manifest commit.
- Plugin registry/versioning.

Exit criteria:

- Wasm resource/security tests.
- Python stream without whole-table collection.
- Object-store failure injection.

### Phase 4 — distributed

- Controller/executor control protocol.
- Heartbeat/lease/fencing.
- Distributed CSV range ingest.
- Ballista adapter.
- Flight remote edge.
- Work stealing/speculative attempt.
- Cluster calibration/placement.

Exit criteria:

- Controller receives no tabular bytes.
- Executor loss recovery.
- Scaling targets trên 2/4/8 nodes.
- TB/min target chỉ công bố nếu benchmark environment đạt.

### Phase 5 — lakehouse and HA

- Iceberg hoặc catalog tương đương.
- Snapshot isolation/schema evolution/vacuum.
- HA controller/consensus hoặc external metadata store.
- Adaptive skew handling và compaction.
- DuckDB plugin.

## 10. Test strategy

### 10.1 Unit

- DAG validation/state transitions.
- Hash/resume contract.
- Encoding/dialect/header scores.
- Quote-aware boundaries.
- Schema normalization/widening.
- Memory permit accounting.
- Manifest conflict/CAS.

### 10.2 Property-based

- Random CSV dialects/quotes/newlines.
- Arbitrary chunk boundaries.
- Decode split tại mọi byte offset.
- Resume/replay equivalence.
- No overlap/no gap partition ownership.

### 10.3 Fuzz

- Malformed encoding.
- CSV parser/boundary scanner.
- Parquet/IPC metadata readers.
- Protobuf control messages.
- Ticket parsing.
- WIT/host-call payloads.

### 10.4 Integration

- CSV → bronze → SQL → silver.
- Repeated/changed headers.
- Reject artifacts.
- DataFusion memory/spill.
- Flight SQL query/cancel.
- S3/R2-compatible mock/real test environment.

### 10.5 Fault injection

- Kill tại từng artifact commit step.
- Lease expiry và stale executor commit.
- Disk full/permission denied.
- Object store timeout/multipart failure.
- Controller restart.
- Network partition/Flight disconnect.
- Python crash/Wasm infinite loop.

### 10.6 Performance

- Criterion microbenchmarks.
- 1M/100M/1B row end-to-end.
- CPU cycles/byte, allocations/row, peak RSS.
- Node profiles và scale-out.
- Baseline comparison trong controlled CI/perf runners.

## 11. Requirements traceability

| Requirement group | Primary design document | Verification |
|---|---|---|
| Orchestration | Architecture + history | State-machine/unit/fault tests |
| Exact resume | History | Crash matrix/property tests |
| CSV uncertainty | CSV ingestion | Fixtures/property/fuzz |
| Big Data/load | Performance | 1B-row and scale benchmarks |
| Storage commit | History + architecture | Fault injection/reconciliation |
| Plugins | Extensibility | Compatibility/security tests |
| Serving | Extensibility | Flight/REST/slow-client tests |
| Security | Extensibility | Auth/fuzz/path/secret tests |

## 12. Risk register

| Risk | Tác động | Mitigation |
|---|---|---|
| Phạm vi thay Airflow+Spark+Kafka quá lớn | Không ship MVP | Giới hạn MVP CSV→Parquet+DAG+history |
| Arrow/DataFusion/Ballista version drift | Compile/type conflicts | BOM, Cargo.lock, upgrade spike |
| Unknown CSV ambiguity | Sai dữ liệu | Bronze-first, raw snapshot, audited decision |
| Resume sai boundary | Loss/duplicate | Safe boundary, contract, property tests |
| Filesystem/DB không atomic chung | Orphans/inconsistency | Intent log + deterministic artifact + reconciler |
| SQLite metadata bottleneck | Scheduler slowdown | Coarse checkpoints, batched events, future external/Raft store |
| Python crash/GIL/thread pools | Process loss/oversubscription | Trust modes, subprocess, thread caps |
| Wasm không zero-copy raw buffers | Throughput thấp | Host handles/kernels, copy only selected data |
| Small files | Query/object-store overhead | Part sizing + compaction |
| Data skew/shuffle | Stragglers/memory | Statistics, adaptive partition, spill/salting |
| Performance target phi thực tế | Sai kỳ vọng | Capacity model và benchmark manifest |
| Low-node disk exhaustion | Run failure | Quota, spill accounting, backpressure/load shedding |

## 13. Open design questions

- Manifest payload dùng Arrow IPC, Protobuf hay hybrid?
- Bronze có lưu `_source_raw_offset` từng row hay chỉ part metadata mặc định?
- Default checkpoint target theo bytes hay time, hoặc cả hai?
- Local raw snapshot ưu tiên reflink/hard-link/copy theo filesystem nào?
- Schema reconciliation nằm ở catalog provider hay custom `TableProvider`?
- Distributed metadata backend đầu tiên sau SQLite là PostgreSQL hay embedded Raft?
- Ballista extension point nào dùng để inject custom source/plugin trên executor?
- Flight SQL client compatibility nào là release target chính?
- Python subprocess protocol dùng Flight hay local IPC shared memory?
- Wasm raw UDF ABI có cần Arrow C Data-like shared memory extension hay chỉ host handles?

Các câu hỏi này không được giải quyết bằng assumption âm thầm; mỗi câu cần spike/ADR trước implementation tương ứng.

## 14. Definition of Done cho MVP

- Tất cả `INV-*` có automated test hoặc runtime assertion/monitor.
- Documentation và pipeline example khớp actual CLI/config.
- Local CSV → Parquet với audited discovery.
- Append-only history và UI/CLI timeline cơ bản.
- Resume sau kill không loss/duplicate.
- Reject/quarantine queryable.
- Peak RSS bounded trong 1B-row test.
- DataFusion SQL task và local spill hoạt động.
- Release binary reproducible từ committed `Cargo.lock`.
- Security baseline: path policy, secret redaction, local auth mode documented.
- Benchmark report chứa hardware, dataset, encoding, codec và all throughput definitions.

## 15. Nguồn kỹ thuật và version references

- [DataFusion 54.1 dependencies/current crate information](https://docs.rs/crate/datafusion/latest)
- [Ballista 53 dependency set](https://docs.rs/crate/ballista/latest)
- [Arrow Flight crate](https://docs.rs/crate/arrow-flight/latest)
- [object_store crate](https://docs.rs/object_store/latest/object_store/)
- [PyO3 crate](https://docs.rs/crate/pyo3/latest)
- [Wasmtime crate](https://docs.rs/wasmtime/latest/wasmtime/)
