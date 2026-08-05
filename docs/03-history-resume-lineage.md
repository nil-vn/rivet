# History, lineage và exact resume

## 1. Mục tiêu correctness

History và resume là lõi của execution model, không phải logging bổ sung. Sau crash hoặc retry, hệ thống phải chứng minh được:

- Input bytes nào đã được xử lý.
- Parser/config/code version nào đã được dùng.
- Output range nào đã commit.
- Attempt nào sở hữu commit.
- Record nào bị reject/quarantine.
- Tại sao scheduler tiếp tục, retry hoặc từ chối checkpoint.

Exact resume được định nghĩa theo final visible dataset:

- Có thể phải reprocess phần chưa checkpoint.
- Không bỏ sót committed logical record.
- Không tạo duplicate visible record.
- Không resume trên source/config/code đã thay đổi.
- Downstream không thấy partial artifact.

## 2. Event-sourced history

### 2.1 Durable entities

| Entity | Identity | Nội dung chính |
|---|---|---|
| `PipelineDefinitionVersion` | pipeline ID + version | Immutable DAG spec và hash |
| `Run` | run ID | Trigger, definition version, state |
| `TaskAttempt` | run + task + attempt | Lease, executor, resource, outcome |
| `TaskEvent` | attempt + sequence | Append-only lifecycle event |
| `SourceSnapshot` | snapshot ID | URI, fingerprint, immutable location |
| `SourceSegment` | segment ID | Raw range, encoding, dialect, header, schema |
| `Artifact` | artifact ID | URI, content hash, range, schema, lineage |
| `Checkpoint` | checkpoint ID | Next safe position và committed artifacts |
| `ManifestGeneration` | dataset + generation | Visible dataset snapshot |
| `RejectArtifact` | artifact ID | Invalid rows và reasons |

### 2.2 Event examples

```text
RunCreated
TaskAttemptCreated
TaskLeased
TaskStarted
SourceSnapshotResolved
EncodingDetected
DialectDetected
HeaderProfileDetected
SchemaVersionCreated
SourceSegmentCreated
PartitionStarted
ArtifactPrepared
ArtifactCommitted
CheckpointCommitted
SchemaDriftDetected
RejectArtifactCommitted
TaskSucceeded
TaskSucceededWithWarnings
TaskFailed
TaskCancelled
LeaseExpired
ManifestPublished
```

Event database không nhận event cho từng row hoặc từng batch nhỏ. Progress được aggregate và checkpoint theo artifact/range.

### 2.3 Event contract

```rust
#[derive(Debug, Clone)]
pub struct TaskEvent {
    pub event_id: String,
    pub run_id: String,
    pub task_id: String,
    pub attempt: u16,
    pub sequence: u64,
    pub occurred_at_micros: i64,
    pub kind: TaskEventKind,
    pub payload_version: u32,
    pub payload: Vec<u8>,
}
```

`payload` dùng Protobuf. Unknown event version phải được preserve ngay cả khi materializer chưa hiểu.

## 3. Task state machine

```text
Pending
   │ dependencies committed
   ▼
Ready ──► Leased ──► Running ──► Committing ──► Succeeded
              │           │             │              │
              │           │             │              └─ terminal
              │           │             └──► RetryWaiting
              │           └────────────────► RetryWaiting
              └────────────────────────────► RetryWaiting
                                                │
                                                ├──► Ready
                                                └──► Failed
```

Terminal states:

- `Succeeded`
- `SucceededWithWarnings`
- `Failed`
- `Cancelled`
- `Skipped`

Transition phải được kiểm soát bằng compare-and-swap trên attempt/generation. Executor không được tự đánh dấu task thành công; nó chỉ báo artifact/commit result, controller xác nhận transition.

## 4. Source snapshot

Mutable path không phải source identity. Snapshot cần:

```rust
#[derive(Debug, Clone)]
pub struct SourceSnapshot {
    pub snapshot_id: String,
    pub original_uri: String,
    pub immutable_uri: String,
    pub byte_length: u64,
    pub modified_at_micros: Option<i64>,
    pub etag: Option<String>,
    pub content_hash: String,
    pub created_at_micros: i64,
}
```

### 4.1 Local MVP

- Immutable input: lưu canonical path, size, mtime và strong hash.
- Mutable/drop folder: reflink/hard-link nếu safe; nếu không, copy vào raw zone.
- Hash được tính trong cùng read pass khi có thể để tránh đọc lại toàn file.
- Trước resume, source snapshot identity phải còn resolve được.

### 4.2 Object storage

- Lưu bucket/key/version ID nếu backend hỗ trợ versioning.
- Lưu size, ETag và strong application hash.
- Không mặc định ETag là content hash đối với multipart upload.
- Dùng conditional read/version constraint khi replay.

## 5. Resume contract

Checkpoint chỉ hợp lệ nếu contract hiện tại bằng contract đã lưu:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeContract {
    pub pipeline_version_hash: String,
    pub task_spec_hash: String,
    pub plugin_name: String,
    pub plugin_version: String,
    pub source_snapshot_hash: String,
    pub parser_config_hash: String,
    pub schema_policy_hash: String,
    pub transform_code_hash: String,
    pub input_artifact_hashes: Vec<String>,
}
```

Contract mismatch dẫn tới:

- Không resume in-place.
- Tạo run mới hoặc fork run.
- Preserve history/artifacts cũ.
- Có thể cache-reuse nếu full artifact key hoàn toàn giống.

Không dùng heuristic để “gần giống thì resume”.

## 6. Checkpoint model

```rust
#[derive(Debug, Clone)]
pub struct PartitionCheckpoint {
    pub checkpoint_id: String,
    pub contract: ResumeContract,
    pub source_snapshot_id: String,
    pub source_segment_id: String,
    pub partition_id: u64,
    pub next_raw_byte_offset: u64,
    pub next_logical_record: u64,
    pub resolved_encoding: String,
    pub dialect_id: String,
    pub header_profile_id: String,
    pub schema_version_id: String,
    pub committed_artifact_ids: Vec<String>,
    pub commit_generation: String,
}
```

Persistent offsets/counters dùng `u64`. In-memory Arrow batch vẫn nhỏ và không liên quan tổng row count.

### 6.1 Safe boundary

Checkpoint chỉ đặt tại:

- Sau closing quote và record terminator hợp lệ.
- Không nằm giữa multibyte character hoặc UTF-16 code unit/surrogate.
- Decoder không giữ partial input sequence.
- Output Parquet part đã đóng footer.
- Artifact đã có deterministic identity và committed state.

Với stateful encoding không có serializable decoder state:

- Resume trên normalized UTF-8 artifact; hoặc
- Replay từ anchor cũ và discard đến committed record.

## 7. Artifact identity và idempotency

Logical artifact key được tạo từ:

```text
hash(
    source_snapshot_id,
    source_segment_id,
    first_raw_offset,
    last_raw_offset,
    schema_version_id,
    transform_code_hash,
    sink_config_hash
)
```

Properties:

- Cùng logical range và cùng contract phải tạo cùng logical key.
- Content hash được tính trên actual file bytes.
- Logical key giống nhưng content hash khác là dấu hiệu nondeterminism/corruption; không tự ghi đè.
- File path chứa logical/content identity, không chỉ attempt number.

Ví dụ:

```text
datasets/customers/
└── schema=81ac.../
    └── source=4b22.../
        ├── range=000000000000-000268435455.parquet
        └── range=000268435456-000536870911.parquet
```

## 8. Commit ordering

Checkpoint không được đi trước sink:

```text
Sai:
    read batch
    save checkpoint
    write Parquet
    crash

Đúng:
    read safe range
    write complete Parquet part
    commit artifact
    commit checkpoint referencing artifact
    publish/advance manifest generation
```

Invariant:

```text
Checkpoint.next_offset = N
```

chỉ hợp lệ nếu toàn bộ logical records trước `N` đã thuộc committed artifacts hoặc được quarantine theo policy.

## 9. Local two-phase artifact protocol

```text
1. Create ArtifactIntent(state=Preparing)
2. Write temporary file under task attempt staging directory
3. Close Parquet writer and footer
4. Flush and fsync file
5. Rename to deterministic immutable path
6. In one SQLite transaction:
   - mark artifact Committed
   - insert checkpoint
   - append ArtifactCommitted and CheckpointCommitted events
   - advance attempt watermark
7. Publish dataset manifest generation
8. Mark task terminal after all required generations commit
```

Filesystem và SQLite không có một distributed atomic transaction chung, nên reconciliation là bắt buộc.

## 10. Crash matrix

| Crash point | Visible state | Recovery action |
|---|---|---|
| Trước `ArtifactIntent` | Không có | Retry range |
| Sau intent, trước temp file | Preparing intent | Mark abandoned/retry |
| Trong khi ghi temp | Partial staging file | Delete after lease expiry |
| Sau footer, trước rename | Complete temp file | Validate rồi rename/reuse |
| Sau rename, trước DB commit | Orphan immutable artifact | Reconcile bằng intent/logical key/hash |
| Sau artifact/checkpoint DB commit | Durable checkpoint | Resume từ next offset |
| Sau checkpoint, trước manifest publish | Artifact chưa visible downstream | Republish manifest idempotently |
| Sau manifest publish | Committed generation | Downstream có thể chạy |

## 11. Object store commit

Object storage không có atomic rename. Protocol:

1. Ghi immutable part object bằng unique deterministic key.
2. Complete multipart upload.
3. Verify size/checksum/metadata.
4. Ghi immutable manifest generation object.
5. Conditional update `current` pointer hoặc catalog generation bằng CAS.
6. Nếu CAS thua, kiểm tra conflict/rebase hoặc abort generation.

Reader chỉ đọc objects được manifest committed tham chiếu.

## 12. Lease và fencing

Lease gồm:

```rust
#[derive(Debug, Clone)]
pub struct TaskLease {
    pub lease_id: String,
    pub run_id: String,
    pub task_id: String,
    pub attempt: u16,
    pub executor_id: String,
    pub fencing_token: u64,
    pub expires_at_micros: i64,
}
```

Mọi final commit phải kèm fencing token. Khi lease được cấp lại, token tăng. Attempt cũ có thể vẫn chạy nhưng commit bị từ chối.

## 13. Resume algorithm

```text
1. Load latest non-terminal run/task attempt state
2. Resolve immutable SourceSnapshot and input artifacts
3. Recompute ResumeContract
4. Reject resume if contract differs
5. Reconcile Preparing/orphan artifacts
6. Load latest monotonically committed checkpoint per partition
7. Verify checkpoint artifacts and manifest references
8. Acquire new lease/fencing token
9. Resume at safe boundary or replay from decoder anchor
10. Reprocess only uncommitted range
11. Commit new immutable artifacts/checkpoints
12. Publish manifest generation
```

## 14. Cache reuse

Cache reuse khác resume:

- Resume: tiếp tục cùng logical run/task contract.
- Cache reuse: run mới tham chiếu artifact cũ có full deterministic key giống.

Cache key phải chứa:

- Input artifact hashes.
- Task payload/SQL/code hash.
- Plugin/version.
- Relevant environment/config.
- Output schema policy.

Nondeterministic task mặc định không cache.

## 15. Reject và warning history

Không ghi rejected row vào control database. Reject được ghi thành Parquet artifact:

```text
_rejects/
└── run=<run-id>/
    └── task=<task-id>/
        └── partition=<id>.parquet
```

Schema tối thiểu:

```text
source_snapshot_id
source_segment_id
raw_byte_offset
logical_record
raw_record
error_code
error_message
column_name
original_value
```

History event chỉ lưu reject count, artifact ID, reason histogram và source range.

## 16. Materialized state và database

SQLite MVP dùng WAL và một serialized writer task. Event append và state transition nằm cùng DB transaction.

Logical tables:

```text
pipeline_definitions
runs
task_attempts
task_events
source_snapshots
source_segments
schema_versions
artifact_intents
artifacts
checkpoints
manifest_generations
executor_leases
```

Index quan trọng:

```text
(run_id, task_id, attempt, sequence) UNIQUE
(run_id, task_id, partition_id, checkpoint_sequence) UNIQUE
(logical_artifact_key) UNIQUE
(dataset_id, generation) UNIQUE
(lease_id, fencing_token) UNIQUE
```

Multi-node không được tăng DB write rate theo batch/row. Checkpoint/event granularity nằm ở part/range level.

## 17. Audit API/CLI

```text
furrumx run history <run-id>
furrumx task history <run-id> <task-id>
furrumx task resume <run-id> <task-id>
furrumx lineage artifact <artifact-id>
furrumx lineage dataset <dataset> --generation <generation>
furrumx dataset versions <dataset>
furrumx reconcile --dry-run
```

API phải trả:

- Timeline ordered theo sequence.
- State transitions.
- Source/checkpoint coordinates.
- Artifact URIs/hashes/schema.
- Retry/lease expiration reason.
- Warning/reject summaries.
- Parent/child lineage graph.

## 18. Retention và garbage collection

- Event history có retention policy nhưng không xóa event cần cho retained manifest lineage.
- Artifact chỉ được xóa khi không còn manifest, checkpoint hoặc legal hold tham chiếu.
- Staging file chỉ xóa sau lease expiry và reconciliation grace period.
- Raw snapshots có policy riêng; xóa raw làm mất khả năng full replay và phải được audit.
- Garbage collection là mark-and-sweep từ committed manifests/checkpoints, không dựa vào filename age đơn thuần.
