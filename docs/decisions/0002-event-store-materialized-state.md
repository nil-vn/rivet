# ADR-0002: Event store và materialized state

- Status: Accepted
- Date: 2026-08-05
- Owners: Project owner and control-plane maintainers
- Deciders: Project owner, control-plane maintainer, storage/recovery maintainer
- Related requirements: `FR-ORCH-006`, `FR-ORCH-008`, `FR-HIST-001`, `FR-HIST-002`, `FR-HIST-008`, `NFR-RES-002`, `NFR-AVL-001`, `NFR-AVL-003`, `NFR-SEC-005`, `INV-009`, `AC-010`
- Related work packages: `WP-020`, `WP-100`, `WP-200`, `WP-220`, `WP-230`
- Related decisions: ADR-0001; ADR-0003 must define artifact/checkpoint/manifest ordering
- Supersedes: N/A

## Context

History là một phần của correctness model, không phải operational logging. Sau retry, controller restart hoặc crash ở commit boundary, hệ thống phải giải thích được:

- Command nào đã được chấp nhận.
- Event nào đã commit và thứ tự durable của chúng.
- Run/task/attempt state nào được suy ra từ history.
- Attempt/fencing generation nào có quyền transition hoặc commit.
- Tại sao retry, cancellation, warning hoặc terminal state xảy ra.
- Materialized state có thể được rebuild hay không.

Nếu chỉ lưu current state, retry có thể ghi đè attempt cũ và mất audit/lineage. Nếu chỉ append event rồi materialize bất đồng bộ, process có thể crash khi event đã commit nhưng current state chưa đổi, khiến scheduler đọc state mâu thuẫn. Nếu event store ghi theo row/batch, metadata write rate và database size sẽ tăng theo dataset thay vì task/partition/artifact.

Local MVP dùng single active controller và SQLite. Distributed/HA phase có thể dùng backend khác, nhưng không được làm yếu ordering, idempotency, fencing hoặc replay semantics.

## Scope

ADR này quyết định:

- Source of truth giữa event ledger, immutable fact ledger và materialized state.
- Event envelope, ordering và payload versioning.
- Atomic append/materialization transaction.
- Idempotent command retry và optimistic concurrency.
- SQLite WAL writer/read model cho local MVP.
- Replay/rebuild, unknown event version và migration behavior.
- Resource, security và retention boundaries của control metadata.

ADR này không quyết định:

- Exact artifact/checkpoint/manifest schema và commit ordering; thuộc ADR-0003.
- Manifest serialization format.
- Local file `fsync`/rename guarantees.
- Business DAG schema hoặc scheduler policy.
- HA leader election, PostgreSQL hay Raft backend.
- Event retention/archival format sau MVP.
- Concrete SQLite Rust crate/version; dependency PR phải review capability, license, maintenance, security, binary size và compile time.

## Decision drivers

- Event append và visible state transition phải atomic.
- Retry không tạo duplicate lifecycle effect.
- Stale attempt/fencing token không được thắng current generation.
- Current run/task/attempt state phải rebuild deterministic từ durable history.
- Unknown event bytes phải được preserve, không bị skip hoặc reinterpret âm thầm.
- Timeline ordered theo durable sequence, không theo wall clock.
- Event rate và memory phải bounded theo command/task/partition/artifact.
- Default local mode không cần external metadata service.
- Backend tương lai có thể thay SQLite mà giữ domain contract.
- Secret, raw record và reject payload không được đi vào control database.

## Options considered

### Option A — Mutable state tables với audit log best-effort

Application update `runs`/`task_attempts` trước rồi ghi audit log sau, hoặc ngược lại.

Ưu điểm:

- CRUD đơn giản.
- Query current state trực tiếp.
- Ít materializer code.

Nhược điểm:

- Crash giữa hai write làm state và history khác nhau.
- Retry dễ overwrite attempt/history cũ.
- Không chứng minh được replay/rebuild.
- Audit log không đủ thẩm quyền cho recovery.

Option này bị từ chối.

### Option B — Pure event sourcing cho mọi durable entity

Chỉ event log là authoritative. Pipeline definition, source snapshot, artifact, checkpoint, manifest và mọi current state đều được dựng lại hoàn toàn từ event payload.

Ưu điểm:

- Một source of truth tuyệt đối.
- Replay/audit model đồng nhất.

Nhược điểm:

- Event payload lớn và duplicate immutable metadata.
- Uniqueness/CAS/foreign-key constraints cho artifact/checkpoint khó enforce hiệu quả.
- Replay toàn bộ history cần thiết cho nhiều operational query.
- Làm ADR này lấn sang storage/manifest/checkpoint format trước khi các contract đó được chốt.

Option này chưa phù hợp MVP và bị từ chối.

### Option C — Hybrid event ledger với synchronous transactional projections

Lifecycle events là append-only ledger. Immutable durable facts có normalized ledger/table riêng và phải link event. Current state là projection được materializer cập nhật trong cùng metadata transaction.

Ưu điểm:

- Giữ audit/replay cho lifecycle mà vẫn dùng database constraints cho durable facts.
- Event và current state không có cửa sổ bất đồng bộ.
- Query scheduler/operator không phải replay history ở normal path.
- Cho phép ADR-0003 mở rộng metadata unit-of-work mà không tạo transaction giả giữa nhiều store calls.

Nhược điểm:

- Cần deterministic materializer và replay tests.
- Event schema và projection schema phải version hóa song song.
- Transaction API phức tạp hơn CRUD trực tiếp.

Option này được chọn.

### Option D — PostgreSQL hoặc external metadata service ngay trong MVP

Ưu điểm:

- Nhiều concurrent writers và remote controller access tốt hơn.
- Operational tooling trưởng thành.

Nhược điểm:

- Thêm external mandatory service, deployment và failure modes.
- Trái single-binary/local MVP goal.
- Không loại bỏ nhu cầu xác định event/projection semantics trước.

Option này bị hoãn tới distributed/HA phase. Backend tương lai phải implement cùng contract hoặc được thay bằng ADR superseding.

## Decision

### 1. Ba loại durable state

Hệ thống phân biệt rõ:

1. **Lifecycle event ledger**: append-only record của run/task/attempt decisions và outcomes.
2. **Immutable fact ledger**: pipeline definition, source snapshot/segment, artifact intent/artifact, checkpoint và manifest generation. Fact schema/ordering cụ thể do domain ADR sở hữu.
3. **Materialized current state**: `Run`, `TaskAttempt`, scheduler/lease summary và projection cursor dùng cho normal query/scheduling.

Lifecycle event ledger là source of truth cho lifecycle. Materialized current state không được update ngoài materializer. Current run/task/attempt state phải rebuild được từ event ledger và supported event decoders.

Immutable fact ledger không phải materialized view. Một fact được coi durable chỉ khi transaction/domain contract của fact đó thỏa; event tương ứng là audit/reference, không thay database constraint hoặc artifact validation. ADR-0003 sẽ định nghĩa fact nào phải commit cùng checkpoint/artifact events.

Operational logs/traces không thuộc ba loại trên và không được dùng cho recovery.

### 2. Event streams và ordering

Mỗi event thuộc một durable stream:

```text
Run stream:         (run_id)
Task-attempt stream:(run_id, task_id, attempt)
```

`TaskId` là durable identity; không persist petgraph `NodeIndex` hoặc process-local identity.

Logical event envelope chứa tối thiểu:

```text
event_id
command_id
stream_kind
stream_id
stream_sequence
commit_revision
event_ordinal
run_id
task_id optional
attempt optional
occurred_at_micros
kind_code
payload_version
payload_bytes
payload_hash
```

Rules:

- `stream_sequence` là `u64`, tăng đơn điệu và unique trong stream.
- `commit_revision` là `u64`, tăng đơn điệu cho mỗi committed metadata transaction.
- `event_ordinal` xác định thứ tự event trong cùng transaction.
- `(commit_revision, event_ordinal)` tạo global replay order trong một controller database.
- `occurred_at_micros` chỉ phục vụ audit/display; clock skew không được thay đổi ordering.
- `event_id` unique toàn database.
- Exact physical ID encoding được định nghĩa bằng canonical encoding/golden tests trong `WP-100`; không ghép unescaped user-controlled string làm identity.

Rust/domain counters dùng `u64`. SQLite adapter có thể dùng non-negative `INTEGER` khi value không vượt `i64::MAX`; conversion phải checked và out-of-range phải trả stable typed error. Không cast/wrap âm thầm.

### 3. Event payload

- Event envelope/query columns là relational fields; event-specific payload là versioned Protobuf bytes.
- `(kind_code, payload_version)` chọn đúng decoder. Protobuf field number đã dùng không được reuse.
- Raw `payload_bytes` và `payload_hash` được preserve nguyên trạng ngay cả khi current binary không hiểu version.
- Protobuf-generated DTO chỉ tồn tại tại serialization boundary; durable domain API dùng owned domain types.
- Materializer decode trực tiếp từ stored bytes và không re-encode để "normalize" history.
- Thêm field mới chỉ giữ cùng `payload_version` khi older materializer có thể bỏ qua mà không đổi state semantics; thay đổi ảnh hưởng transition/projection phải bump version.
- Writer chỉ emit kind/version mà materializer cùng binary hiểu.
- Event payload không chứa plaintext secret, raw record, tabular batch hoặc unbounded reject list.
- Reject rows nằm trong reject artifact; event chỉ chứa count, bounded histogram, ranges và artifact IDs.
- Mỗi event và mỗi command có hard byte/count limit. Queue reservation tính cả envelope và payload bytes.

Vì local history dùng Protobuf, implementation `WP-200` phải làm `prost` khả dụng cho local history profile mà không kéo Ballista/Tonic vào minimal `--no-default-features`. Thay đổi Cargo feature mapping phải được review cùng ADR-0001 compatibility evidence.

### 4. Metadata command và idempotency

Mọi mutation đi qua typed metadata command, không expose arbitrary SQL transaction cho caller. Command envelope chứa tối thiểu:

```text
command_id
command_hash
bounded stream appends, mỗi append có expected stream sequence
expected projection revisions/assertions
optional expected fencing token/generation
typed fact mutation input when another ADR allows it
```

Một command có thể atomically append vào nhiều stream, ví dụ hoàn tất attempt và advance run state. Số stream/event và tổng bytes vẫn phải bounded. Writer gán `event_id`, sequence, commit revision, ordinal và timestamp ở lần execute đầu tiên; exact retry trả receipt cũ thay vì tạo identity/timestamp mới.

`command_hash` dùng canonical encoding của semantic command, gồm ordered stream appends, expected revisions/fencing assertions, event kind/payload hashes và typed fact inputs. Hash không gồm `command_id`, writer-assigned fields hoặc plaintext secret. Exact canonical format và algorithm thuộc `WP-100` và phải có golden tests.

Writer xử lý retry như sau:

- Chưa có `command_id`: validate và execute command.
- Đã có cùng `command_id` và cùng `command_hash`: trả lại durable `CommitReceipt`; không append event lần hai.
- Đã có cùng `command_id` nhưng khác hash: fail `HISTORY_IDEMPOTENCY_CONFLICT`.
- `event_id` duplicate ngoài exact command retry: fail conflict.
- Expected sequence/state revision không khớp: fail compare-and-swap; không append gì.
- Fencing token/generation stale: fail; không append gì.

`CommitReceipt` chứa `command_id`, committed revision, event/stream sequence ranges và projection revisions cần thiết để read-after-write. Receipt được persist trong cùng transaction.

### 5. Atomic metadata transaction

Local writer thực hiện một command theo thứ tự logic:

```text
1. Validate command size/schema outside the write transaction where safe
2. BEGIN IMMEDIATE
3. Resolve exact idempotency retry/conflict or insert command ledger row
4. Validate expected stream sequence/state/fencing assertions
5. Allocate commit revision
6. Insert typed immutable facts allowed by the command/domain contract
7. Append event rows
8. Apply events through deterministic materializer
9. Persist projection cursor and command receipt
10. COMMIT
11. Acknowledge caller
```

Steps 3–9 nằm trong cùng SQLite transaction. Invalid event transition, constraint error, cancellation trước commit hoặc I/O error phải rollback toàn transaction. Không acknowledge trước successful commit.

Transaction không thực hiện filesystem/network I/O, parsing hoặc long CPU work. Cancellation sau `COMMIT` nhưng trước acknowledgment không được undo durable state; caller retry cùng `command_id` để lấy receipt đã commit.

Thứ tự statements bên trong transaction không tự quyết artifact/checkpoint semantics. ADR-0003 phải quy định preconditions và fact mutations cho artifact/checkpoint/manifest command. Không mô phỏng atomicity bằng ba lời gọi độc lập tới storage, checkpoint và history stores.

Minimum logical tables của ADR này:

```text
history_events
history_commands
runs_current
task_attempts_current
projection_metadata
schema_migrations
```

`history_commands` lưu idempotency hash và commit receipt. `projection_metadata` lưu materializer/schema version, last applied revision và rebuild status/hash. Fact tables được ADR/domain owner bổ sung nhưng phải tham gia cùng metadata database transaction khi contract yêu cầu.

Minimum uniqueness/order constraints:

```text
event_id UNIQUE
(command_id) FOREIGN KEY history_commands
(stream_kind, stream_id, stream_sequence) UNIQUE
(commit_revision, event_ordinal) UNIQUE
command_id UNIQUE with immutable command_hash
run_id PRIMARY KEY in runs_current
(run_id, task_id, attempt) PRIMARY KEY in task_attempts_current
```

Payload BLOB không được index. Audit query dùng denormalized run/task/attempt columns và durable sequence/revision indexes; exact index set phải được đo bằng representative metadata workload.

### 6. Materializer

- Materializer là deterministic function theo event kind/version: `(previous_state, event) -> next_state`.
- Invalid state transition aborts transaction với stable error; không append audit event cho một transition chưa commit.
- Database `UNIQUE`, `CHECK`, foreign-key và state revision constraints là defense-in-depth; business transition logic không được duplicate khác nghĩa trong ad-hoc SQL trigger.
- Projection row ghi revision/event cursor cuối đã apply.
- Scheduler và operator APIs đọc projection/fact tables, không replay event log trong normal request path.
- Read sau returned `CommitReceipt` phải quan sát transaction đó khi query với minimum revision tương ứng; adapter phải mở snapshot mới hoặc bounded-retry thay vì dùng read transaction cũ.
- Read transaction phải ngắn; long-lived reader gây WAL checkpoint starvation phải có metric/cancellation policy.

Materialized tables ban đầu gồm logical `runs_current`, `task_attempts_current` và `projection_metadata`. Lease projection/schema cụ thể thuộc `WP-220`; durable fact tables thuộc owning domain/ADR.

### 7. SQLite local backend

Local MVP dùng một SQLite database trên local filesystem với:

- `journal_mode=WAL`; startup phải verify SQLite thực sự chuyển/đang ở WAL mode.
- `synchronous=FULL` cho durable default.
- `foreign_keys=ON` trên mọi connection.
- Một single-writer worker sở hữu write connection.
- Writer input queue bounded theo command count và bytes; producer nhận backpressure.
- Blocking SQLite work chạy ngoài Tokio async workers; không giữ blocking lock qua `.await`.
- Bounded read connections và short snapshot transactions.
- Bounded busy retry/deadline; không retry vô hạn.
- WAL size/checkpoint latency/busy count metrics và explicit checkpoint policy để tránh WAL growth không giới hạn.

SQLite WAL database phải nằm trên cùng host/local filesystem được policy cho phép. Không đặt control database trên network filesystem. Main database và `-wal` chứa persistent recovery state; `-shm` là WAL index do SQLite quản lý. Backup/copy không được lấy riêng main database khi WAL đang active: dùng SQLite backup API hoặc controlled checkpoint/close, và không tự xóa/copy sidecar files ad hoc.

MVP có một active application writer. Tooling/migration/reconcile command không được mở writer cạnh tranh mà không đi qua exclusive maintenance mode.

Mỗi logical command là một atomic transaction. Writer có thể batch nhiều events của cùng command. Grouping nhiều independent commands vào một transaction chỉ được thêm sau benchmark/fault evidence và phải giữ riêng idempotency receipt, error attribution và visibility semantics của từng command.

### 8. Replay, rebuild và unknown versions

Startup normal path:

1. Verify database schema/materializer compatibility.
2. Verify projection cursor không vượt event ledger revision.
3. Apply supported events sau cursor nếu projection lag do approved migration/rebuild flow.
4. Validate projection invariants trước readiness.

Projection rebuild:

- Event ledger không bị rewrite.
- Rebuild dùng global `(commit_revision, event_ordinal)` order.
- Rebuild materialized state vào shadow tables trong maintenance mode.
- Validate terminal transitions, row counts, final revisions và deterministic state hashes.
- Chỉ replace/swap current projections sau validation trong transaction.
- Re-running rebuild phải cho cùng final state.

Khi gặp unknown `kind_code` hoặc `payload_version`:

- Preserve raw row/bytes/hash.
- Dừng materialization tại event trước unknown event; có thể scan tiếp chỉ để diagnostic nhưng không publish rebuilt projections.
- Không skip unknown event rồi apply event sau nó.
- Trả `HISTORY_UNKNOWN_EVENT_VERSION` với non-sensitive event metadata.
- Toàn controller database không writable-ready; unknown event không được mặc định là ignorable.
- Raw audit/export có thể tiếp tục trong read-only mode.

Older binary phải fail writable startup khi database schema/materializer version mới hơn supported range; không downgrade projection âm thầm.

### 9. Migration, retention và corruption handling

- Schema migrations có ordered version, checksum và transaction boundary rõ.
- Migration không rewrite historical payload để "nâng version"; materializer hỗ trợ declared historical versions hoặc migration bị block.
- Event/fact/projection corruption hoặc hash mismatch fail startup/rebuild rõ ràng; không bỏ row.
- MVP không delete lifecycle events. Retention/archival chỉ được thêm bằng ADR, và không được xóa history cần cho retained manifest/checkpoint/lineage.
- Database file permissions, backup và diagnostic export phải bảo vệ tenant, source URI, lineage và operational metadata.
- Error/log không dump `payload_bytes`, resolved secrets hoặc raw values.

### 10. Port ownership

- `history` sở hữu event envelope, event-store port, materializer, replay và history query model.
- `core` sở hữu durable IDs, revisions/generations, canonical hashing và stable shared errors.
- `control` sở hữu command orchestration và commit authorization.
- SQLite adapter nằm trong `history`; runtime tạo/wire writer/readers.
- `checkpoint`/`storage` không tự mutate history projection. ADR-0003 cung cấp typed metadata command/fact participant qua `control` coordinator.
- CLI/serving chỉ gọi history/control facade; không mở SQLite trực tiếp.

## Consequences

### Positive

- Crash không thể để lifecycle event committed nhưng current state chưa đổi, hoặc ngược lại.
- Exact command retry không duplicate events/transitions.
- Scheduler có query path nhanh nhưng state vẫn replay/audit được.
- Stale attempt bị chặn tại cùng transaction với state/event mutation.
- SQLite giữ local MVP self-contained; backend future có contract rõ để tương thích.
- Unknown event version không gây silent history loss.

### Negative

- Phải duy trì event decoder/materializer cho historical versions.
- Projection schema và replay tooling làm tăng implementation/test complexity.
- `synchronous=FULL` và one-writer architecture tăng commit latency; coarse event granularity là bắt buộc.
- SQLite writer throughput giới hạn metadata scale của MVP.
- WAL checkpointing, backup và long-reader behavior cần operational metrics/runbook.
- Local feature profile cần Protobuf runtime và SQLite dependency sau dependency review.
- Chưa có production event database nên không cần data migration cho decision ban đầu; sau migration đầu tiên, envelope/schema change phải có compatibility plan.

### Follow-up work

- ADR-0003: artifact/checkpoint/manifest metadata unit-of-work và crash ordering.
- ADR cho memory permits: writer queue byte accounting và admission integration.
- `WP-100`: durable IDs, command/revision newtypes, hashes và stable errors.
- `WP-200`: migrations, SQLite adapter, event codecs, materializer, replay/rebuild.
- `WP-220`: state machine, leases/fencing và controlled clock tests.
- `WP-230`: timeline/lineage CLI/API và redacted export.
- HA/backend ADR phải map transaction, ordering, idempotency và replay semantics tương đương.

## Verification

Decision đã được accepted; điều này không đồng nghĩa implementation hoàn tất. `WP-200` không được đóng cho tới khi có evidence sau.

### Unit and property tests

- Valid/invalid transition matrix cho mọi run/task/attempt state.
- Event codec golden tests theo `(kind_code, payload_version)`.
- Canonical stream/command/event IDs và hash golden tests.
- Exact idempotency retry, mismatched command hash và duplicate event/sequence.
- Checked persistent counter conversion, không wrap/truncate.
- Materializer determinism với randomized valid event sequences.

### Integration and fault tests

- Crash/fault trước insert, sau event insert, sau projection update và trước/sau commit.
- Sau restart chỉ thấy cả event+projection hoặc không thấy cả hai.
- Concurrent commands trên cùng stream: một CAS thắng, command còn lại retry/fail rõ ràng.
- Stale fencing token không append event hoặc mutate state.
- Controller restart replay ra cùng projection hash/current state.
- Unknown event version được preserve và blocks writable readiness.
- Migration/rebuild shadow tables idempotent; failed validation không replace current state.
- Disk full, permission denied, corrupt WAL/database và bounded busy timeout.
- Secret/raw reject sentinel không xuất hiện trong event payload/log/error/export.

### Resource and performance tests

- Writer queue có hard command/byte bounds và propagates backpressure.
- Event rate tăng theo task/partition/artifact, không theo row/batch.
- Báo commit latency p50/p95/p99, writer CPU, WAL/checkpoint bytes/latency và database growth.
- Long reader/checkpoint-starvation test chứng minh WAL không tăng không giới hạn trong supported policy.
- Benchmark durability dùng `synchronous=FULL`; mode yếu hơn không đại diện default.

### Required quality gates

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Feature/dependency PR phải bổ sung minimal/default feature checks và không kéo distributed/Python/Wasm dependencies vào `--no-default-features`.

## References

- [History, lineage và exact resume](../03-history-resume-lineage.md)
- [Kiến trúc hệ thống](../02-system-architecture.md)
- [Kế hoạch phát triển chi tiết](../08-development-plan.md)
- [SQLite Write-Ahead Logging](https://www.sqlite.org/wal.html)
- [SQLite transactions](https://www.sqlite.org/lang_transaction.html)
- [SQLite synchronous pragma](https://www.sqlite.org/pragma.html#pragma_synchronous)
- [SQLite isolation](https://www.sqlite.org/isolation.html)
