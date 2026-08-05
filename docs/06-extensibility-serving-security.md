# Extensibility, serving, vận hành và bảo mật

## 1. Plugin architecture

Plugin được chia thành:

- Data source.
- Sink/storage.
- Transform/UDF.
- Python model.
- Wasm component.
- Authentication/authorization provider.
- Catalog/metadata provider.

Mỗi plugin có descriptor:

```rust
#[derive(Debug, Clone)]
pub struct PluginDescriptor {
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub artifact_hash: String,
    pub capabilities: Vec<String>,
    pub deterministic: bool,
    pub trusted: bool,
}
```

Registry reject:

- Duplicate name/version conflict.
- Unsupported API version.
- Missing capability dependencies.
- Invalid signature/hash.
- Plugin policy không phù hợp executor.

## 2. Rust-native plugins

Rust trait object không có stable ABI xuyên dynamic library. Do đó:

- Built-in Rust plugins được compile vào binary.
- Optional plugins dùng Cargo features.
- Không load arbitrary `.so` rồi truyền `dyn Trait` qua ABI.
- Native plugin độc lập process giao tiếp bằng Flight/control protocol versioned.
- Runtime third-party extension ưu tiên Wasm.

Registration có thể dùng explicit module bootstrap hoặc compile-time inventory, nhưng phải deterministic và test được.

## 3. Python/PyO3 integration

### 3.1 Data contract

Python plugin nhận/trả `pyarrow.RecordBatchReader` hoặc Arrow stream, không nhận Pandas mặc định:

```python
def transform(reader, context):
    for batch in reader:
        yield apply_model(batch, context)
```

Rust dùng Arrow C Data/C Stream Interface để chia sẻ buffers trong cùng process. Không `collect()` toàn input thành `pyarrow.Table` trừ khi plugin contract yêu cầu và resource admission cho phép.

Ưu tiên:

1. PyArrow stream.
2. Polars Arrow-compatible conversion.
3. Pandas compatibility adapter.

Pandas không được quảng bá zero-copy vì string/null/dtype conversion có thể allocate.

### 3.2 Model lifecycle

- Model load một lần theo executor/model hash.
- Cache có memory quota và eviction.
- Model version/hash nằm trong resume contract.
- Pin thread counts cho BLAS/OpenMP/model runtime.
- Warm-up tách khỏi measured task execution nhưng vẫn có history.
- Batch size theo model profile, không mặc định theo CSV batch.

### 3.3 Trust modes

```text
trusted-inprocess
    PyO3 trực tiếp, nhanh nhất, crash native extension có thể hạ process

trusted-subprocess
    cùng executable khởi động Python worker child, trao đổi Arrow Flight/IPC

disabled
    executor không advertise python capability
```

Python interpreter và wheels làm binary/package lớn, có platform dependency. Vì vậy `python` là optional build feature. “Single binary” với bundled Python chỉ là distribution profile riêng, không phải minimal artifact.

## 4. Wasm integration

### 4.1 Component boundary

Wasm linear memory tách khỏi host memory. Zero-copy raw Arrow buffers không portable. Thiết kế dùng host-managed resource handle:

```wit
package furrumx:analytics;

interface batches {
    resource batch;

    schema: func(input: borrow<batch>) -> list<u8>;
    row-count: func(input: borrow<batch>) -> u64;
}

world transform-plugin {
    import batches;

    export transform: func(
        input: batches.batch
    ) -> result<batches.batch, string>;
}
```

Hai execution path:

- Handle path: Wasm orchestration, host thực hiện Arrow kernels; không copy batch vào guest.
- Raw UDF path: copy selected primitive buffers/IPC vào guest memory; linh hoạt nhưng tốn copy.

### 4.2 Sandbox

- Filesystem/network deny-by-default.
- Explicit WASI preopens/capabilities.
- `ResourceLimiter` giới hạn memory/table/instances.
- Fuel cho deterministic compute budget.
- Epoch deadline cho wall-clock interruption.
- Async host calls với timeout.
- Host-call payload limit.
- Compiled component cache theo content hash.
- Per-invocation cancellation.

Fuel/epoch không tự hủy blocking host call; host I/O phải async và được timeout riêng.

## 5. Storage extensibility

`object_store` là abstraction thống nhất cho:

- Local filesystem.
- In-memory tests.
- S3.
- Cloudflare R2 qua S3-compatible endpoint.
- GCS/Azure/HTTP khi bật feature.

Registry ánh xạ URL scheme/authority tới store instance. Credentials được resolve từ secret provider, không embed vào URI/history.

### 5.1 R2/S3 configuration

- Custom endpoint.
- Region/compatibility options.
- Path-style addressing khi cần.
- Multipart threshold/concurrency.
- Conditional put support detection.
- Retry/backoff/rate limit.
- TLS root configuration.

### 5.2 DuckDB future path

- In-process Arrow C Data Interface cho batch/table exchange.
- DuckDB catalog/SQL là optional plugin, không thay DataFusion core.
- Version/ABI và native library packaging phải tách khỏi core MVP.
- Nếu chạy out-of-process, dùng Flight/IPC thay vì pointer sharing.

## 6. Flight SQL serving

### 6.1 Query flow

```text
Flight SQL command
    → authenticate
    → authorize catalog/table/columns
    → parse and plan with DataFusion
    → admission control
    → create query state and signed partition tickets
    → return FlightInfo
    → DoGet(ticket)
    → stream RecordBatch
    → cancellation/cleanup
```

### 6.2 Required methods

MVP read-only:

- Statement query.
- `GetFlightInfo`/`GetSchema`.
- `DoGet`.
- Catalog/schema/table/type metadata.
- Prepared statement create/close/execute.
- Query cancellation.
- SQL info/capability metadata.

Transactions/updates chỉ được advertise nếu catalog/sink thực sự hỗ trợ semantics đó.

### 6.3 Tickets

Ticket không chứa raw SQL. Payload/reference:

```rust
#[derive(Debug, Clone)]
pub struct QueryTicketClaims {
    pub query_id: String,
    pub partition_id: u64,
    pub tenant_id: String,
    pub subject_id: String,
    pub issued_at_micros: i64,
    pub expires_at_micros: i64,
    pub nonce: String,
}
```

- Ký HMAC/asymmetric theo deployment.
- TTL ngắn.
- Bind tenant/subject.
- Có thể one-time hoặc replay-limited.
- Server vẫn re-authorize trên `DoGet`.

### 6.4 BI compatibility

Không giả định mọi BI client triển khai Flight SQL giống nhau. Compatibility suite phải kiểm:

- JDBC/ODBC driver version.
- Metadata calls.
- Prepared statements.
- Timestamp/timezone/decimal types.
- Cancellation.
- TLS/auth.
- Large result streaming.

## 7. REST và WebSocket

Endpoints:

```text
POST   /v1/queries
GET    /v1/queries/{id}
DELETE /v1/queries/{id}
GET    /v1/queries/{id}/stream
GET    /v1/runs/{id}
GET    /v1/runs/{id}/history
POST   /v1/pipelines/{id}/runs
WS     /v1/queries/{id}/ws
GET    /health/live
GET    /health/ready
GET    /metrics
```

External control JSON được phép; tabular response dùng:

```http
Content-Type: application/vnd.apache.arrow.stream
```

WebSocket frames:

1. Protocol/version handshake.
2. Arrow schema IPC.
3. RecordBatch IPC frames.
4. Progress/metrics control frames.
5. End/error frame.

Slow consumer bị giới hạn bởi byte-accounted send queue. Khi queue đầy, query execution nhận backpressure hoặc result được materialize theo explicit policy.

## 8. Authentication và authorization

### 8.1 Authentication

- TLS bắt buộc ngoài local development.
- mTLS cho inter-node deployments có yêu cầu cao.
- OIDC/JWT hoặc external bearer token.
- Token validation trên từng request/RPC.
- Clock skew và key rotation handling.

### 8.2 Authorization

Actions:

```text
pipeline.read
pipeline.write
pipeline.run
run.read
run.cancel
dataset.read
dataset.write
query.execute
plugin.install
admin.executor
```

Data access policy áp dụng trước physical planning:

- Catalog/schema/table grants.
- Column masking.
- Row filters.
- Export limits.
- Tenant isolation.

SQL parser không phải authorization layer.

## 9. Multi-tenancy và quotas

Per tenant/user:

- Concurrent runs/queries.
- Vcores.
- Reserved/peak memory.
- Temporary disk.
- Source/sink bandwidth.
- Result bytes/rows.
- Plugin capabilities.
- Query/run deadline.

Resource class priority không được bypass hard quota.

## 10. Secrets

- Pipeline spec chứa secret reference, không chứa plaintext value.
- Event/history log reference ID, không log resolved secret.
- Redaction trong tracing/error.
- Credentials lifetime/rotation.
- Executor chỉ nhận secret cần cho task lease hiện tại.
- Wasm/Python không nhận ambient secrets.

## 11. File and URI security

- Canonicalize local paths.
- Allowlisted roots.
- Chặn `..`, symlink escape và arbitrary `/proc`/device access.
- Remote users không được tự chọn `file://` ngoài policy.
- HTTP source có SSRF allowlist/denylist, redirect limit và DNS/IP policy.
- Object keys không được ghép trực tiếp từ untrusted column values mà không sanitize.

## 12. Plugin security

- Signed/hashed artifact.
- API compatibility check.
- Capability declaration.
- Trusted flag không do plugin tự cấp.
- Wasm limits per store/invocation.
- Python disabled cho untrusted tenant.
- Native plugin panic/crash blast radius được ghi rõ.
- Dependency/SBOM và vulnerability scan cho release.

## 13. Query and parser security

- Max CSV record bytes.
- Max columns/header depth.
- Max nested Arrow type depth.
- Parquet/IPC metadata limits.
- SQL statement length/complexity limits.
- Planning timeout.
- Cross join policy.
- Regex/UDF execution limit.
- Decompression bomb protection.
- Fuzzing malformed CSV/Parquet/IPC/control payloads.

## 14. Observability

### 14.1 Structured tracing

Spans:

```text
run
task_attempt
source_discovery
source_partition
decode
parse
query_plan
query_stage
parquet_write
artifact_commit
manifest_publish
flight_stream
plugin_invocation
```

### 14.2 Metrics

- Scheduler queue/lease/retry.
- Data stage bytes/rows/rate.
- Memory permits/RSS/spill.
- Object store requests/retries/range bytes.
- Plugin cold/warm invocation.
- Flight SQL latency/result bytes/cancellation.
- Auth failures/rate-limit/load-shed.
- History/event/checkpoint/commit latency.

### 14.3 Logs và audit

- Operational logs có thể JSON ở system boundary.
- Internal tabular data không đi qua JSON.
- Audit log tách operational debug log.
- Không log raw records/secrets mặc định.
- Sampling log không được làm mất task/event history.

## 15. Health và graceful shutdown

Liveness:

- Process/runtime còn hoạt động.

Readiness:

- State store sẵn sàng.
- Required object stores resolve được.
- Listener bound.
- Controller/executor role initialized.

Graceful shutdown:

1. Ngừng nhận work/query mới.
2. Mark executor draining.
3. Hủy hoặc chờ active interactive queries theo deadline.
4. Flush/abort writers an toàn.
5. Commit checkpoint chỉ khi artifact hoàn chỉnh.
6. Release leases hoặc để expire có kiểm soát.
7. Close telemetry.

## 16. Tài liệu tham khảo kỹ thuật

- [Arrow PyArrow integration](https://arrow.apache.org/rust/arrow/pyarrow/index.html)
- [Arrow C Data Interface](https://arrow.apache.org/docs/13.0/format/CDataInterface.html)
- [Wasmtime Store/resource limits](https://docs.rs/wasmtime/latest/wasmtime/struct/Store.html)
- [Arrow Flight SQL](https://arrow.apache.org/docs/format/FlightSql.html)
- [object_store](https://docs.rs/object_store/latest/object_store/)
