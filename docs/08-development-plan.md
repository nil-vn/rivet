# Kế hoạch phát triển chi tiết

## 1. Mục đích và phạm vi

Tài liệu này chuyển architectural baseline trong `docs/01-*` tới `docs/07-*` thành một kế hoạch delivery có thể giao cho maintainer, coding agent và OSS contributor. Đây là execution plan, không thay thế requirement hoặc ADR.

Thứ tự ưu tiên khi có mâu thuẫn:

1. `INV-*` và correctness/security requirements.
2. ADR đã được chấp nhận.
3. Functional/non-functional requirements.
4. Tài liệu kiến trúc domain.
5. Kế hoạch này.

Mọi estimate là khoảng effort để lập capacity, không phải deadline. Throughput 100+ GB/phút/node hoặc TB/phút/cluster chỉ trở thành release claim sau khi vượt benchmark gate trên hardware envelope được công bố.

## 2. Kết quả delivery

Kế hoạch tạo ra sáu release outcomes tăng dần:

| Release | Outcome | Phạm vi chính |
|---|---|---|
| `R0` | Engineering foundation | BOM, toolchain, CI, ADR, benchmark/fault harness |
| `R1` | Local ingestion alpha | CSV bất định → audited Bronze Parquet, bounded memory |
| `R2` | Resilient local MVP | Exact resume, history/lineage, DAG, SQL transform, CLI |
| `R3` | Serving and extension beta | Flight SQL, REST/WS, auth, Wasm/Python, S3/R2 |
| `R4` | Distributed beta | Controller/executor, distributed ingest, Ballista, Flight edge |
| `R5` | Production candidate | Hardening, compatibility, scale evidence, release/security process |
| `R6` | Lakehouse/HA evolution | Snapshot catalog, HA metadata, compaction, DuckDB option |

`R1` là ingestion MVP theo nghĩa hẹp. Product MVP đầy đủ theo Definition of Done trong `docs/07-project-structure-delivery.md` là `R2`.

## 3. Nguyên tắc triển khai

### 3.1 Correctness-first vertical slices

Mỗi slice phải đi hết một đường dữ liệu nhỏ nhưng thật:

```text
input snapshot
  → bounded batch stream
  → immutable output artifact
  → checkpoint/history
  → query/inspection
```

Không xây parser tốc độ cao tách rời commit semantics quá lâu. Không xây distributed scheduler trước khi local lease, fencing và recovery đã có fault tests.

### 3.2 Bounded-by-construction

Mọi work package tạo queue, cache, worker pool, writer hoặc stream phải định nghĩa:

- Byte limit.
- Concurrency limit.
- Ownership của memory permit.
- Backpressure propagation.
- Cancellation/drop behavior.
- Temporary disk quota.
- Metrics cho current/peak usage.

Review không chấp nhận “sẽ thêm limit sau” trên production path.

### 3.3 Contract before implementation

Các contract sau phải được version hóa trước khi có nhiều producer/consumer:

- Pipeline definition.
- Event payload.
- Resume contract.
- Artifact/manifest format.
- Plugin descriptor/API.
- Control-plane Protobuf.
- Flight/REST ticket.

### 3.4 Evidence-driven performance

Một optimization chỉ hoàn tất khi có:

- Bottleneck/profile trước thay đổi.
- Baseline và candidate trên cùng environment.
- Correctness-equivalent output.
- Throughput, CPU, peak RSS/allocation và output size.
- Raw samples/statistics và benchmark manifest.
- Complexity/portability assessment.

Policy chi tiết nằm tại `docs/development/performance-quality-gates.md`.

### 3.5 Feature isolation

Feature profile được phát triển theo thứ tự:

```text
minimal-local
  → full-local
  → s3
  → wasm
  → python
  → distributed
```

Core local build không phụ thuộc Python interpreter, Wasmtime hoặc Ballista. Tất cả profile phải dùng một Arrow/DataFusion type universe tương thích.

## 4. Dependency graph và critical path

```text
WP-001 Repository/CI ───────┐
WP-010 BOM spike ───────────┼──► WP-100 Core contracts
WP-020 ADR baseline ────────┘             │
                                          ├──► WP-110 Resource accounting
                                          ├──► WP-200 Event/history store
                                          └──► WP-300 Source snapshot

WP-110 ─► WP-120 Local transport ───────────────┐
WP-200 ─► WP-210 DAG/state ─► WP-220 Scheduler ─┼──► WP-410 Artifact/checkpoint
WP-300 ─► WP-310 Discovery ─► WP-320 Framing ───┤
                               WP-330 Parser ───┤
WP-400 Parquet sink ────────────────────────────┘
                         │
                         ▼
               WP-430 Local end-to-end
                         │
             ┌───────────┼────────────────────┐
             ▼           ▼                    ▼
       WP-500/510 SQL  WP-620 S3/R2   WP-630 Plugin registry
             │           │                    │
             ▼           │              ┌─────┴─────┐
       WP-520 QoS         │              ▼           ▼
        ├──► WP-530 CLI   │         WP-640 Wasm  WP-650 Python
        ├──► WP-600 Flight SQL
        └──► WP-610 REST/WS
                         │
             WP-520 + WP-620 + WP-630
                         │
                         ▼
                WP-700 Control protocol
                         │
            ┌────────────┼────────────┐
            ▼            ▼            ▼
      WP-710 Leases  WP-720 Dist.  WP-740 Ballista
                     ingestion
            └────────────┼────────────┘
                         ▼
                  WP-760 Scale gate
                         │
                         ▼
               WP-800 Lakehouse/HA
```

Critical path tới `R2`:

```text
BOM → core IDs/contracts → event store → DAG/scheduler
    → source snapshot/discovery/parser → Parquet artifact
    → checkpoint/reconciliation → end-to-end recovery
    → DataFusion SQL → product MVP gate
```

Critical path tới `R4` bổ sung:

```text
object-store commit → stable control protocol → lease/fencing
    → distributed ingest/Flight edge → Ballista adapter → scale/failure gate
```

## 5. Work-package contract

Mỗi work package phải được mở thành một tracking issue có tối thiểu:

| Field | Nội dung bắt buộc |
|---|---|
| Objective | Điều gì trở thành đúng sau khi hoàn tất |
| Scope | Module/file/API thuộc ownership của package |
| Non-goals | Những gì chủ ý chưa giải quyết |
| Dependencies | WP/ADR/API cần có trước |
| Requirements | `FR-*`, `NFR-*`, `INV-*`, `AC-*` liên quan |
| Deliverables | Code, migration, fixture, benchmark, docs |
| Acceptance | Điều kiện nhị phân để đóng issue |
| Verification | Commands, tests, fault scenarios, benchmark |
| Performance class | `P0-HOT`, `P1-SCALE`, `P2-LATENCY`, `P3-NEUTRAL` |
| Risks | Correctness, compatibility, security, performance |
| Handoff | Theo `docs/development/agent-handoff.md` |

Một WP lớn được chia thành PR theo thứ tự ưu tiên:

1. Contract/types/tests hoặc benchmark baseline.
2. Storage/schema/migration nếu có.
3. Implementation focused.
4. Integration/fault tests.
5. Optimization có profile.
6. Documentation/changelog.

PR không nên vừa thay durable format, vừa tối ưu hot path, vừa refactor unrelated modules.

## 6. Phase 0 — repository và technical-risk retirement

### Mục tiêu

Khóa những quyết định có thể làm toàn bộ implementation không compile hoặc buộc rewrite: dependency universe, MSRV, durable formats, memory model và test/benchmark infrastructure.

### `WP-000` — OSS/legal readiness

**Deliverables**

- Chủ dự án chốt tên, LICENSE và DCO/CLA/repository-license model.
- Public maintainer/security/conduct contacts.
- CODEOWNERS và protected-branch policy khi repository public.
- Dependency license/provenance policy.

**Acceptance**

- Các blocker tương ứng trong `docs/community/open-source-readiness.md` được đóng.
- External code contributions chỉ mở sau khi contribution terms rõ ràng.

WP này không block private/internal engineering, nhưng block public code intake và release.

### `WP-001` — Rust project bootstrap

**Deliverables**

- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `build.rs`.
- `src/main.rs`, `src/lib.rs`, module skeleton tối thiểu.
- Lint policy, formatting, feature profile và release profile.
- CLI `rivet --version`, `rivet doctor`, structured tracing bootstrap.
- Clean-clone developer setup.

**Acceptance**

- `cargo fmt --all --check`, `cargo check --workspace --all-targets`, Clippy và empty test suite pass.
- Minimal binary không link Python, Wasmtime hoặc Ballista.
- `Cargo.lock` được commit và build dùng `--locked`.

### `WP-010` — dependency/BOM compile spikes

**Deliverables**

- Compile matrix cho local, Flight SQL, Wasm, Python và distributed features.
- `cargo tree` evidence cho Arrow/DataFusion/Ballista/PyO3 type universe.
- Spike DataFusion `RecordBatch` → Parquet.
- Spike Ballista remote query.
- Spike PyArrow C Stream round trip.
- Spike Wasmtime component resource limits.
- ADR cho BOM/MSRV/feature policy.

**Acceptance**

- Không có incompatible Arrow major universe trong từng supported profile.
- Mỗi spike có minimal test hoặc reproducible command.
- Không đưa spike code không production-ready vào core path nếu chưa review.

### `WP-020` — ADR baseline

Tạo và chấp nhận tối thiểu:

1. BOM và MSRV.
2. Business DAG/physical DAG separation.
3. Event store/materialized-state model.
4. Checkpoint-after-artifact invariant.
5. Manifest format/versioning.
6. Local fsync/rename durability contract.
7. Memory permit/accounting model.
8. Raw/Bronze/Silver semantics.

Open question chưa đủ evidence phải có spike owner và decision deadline theo milestone, không để implicit trong code.

### `WP-030` — CI, fixtures và benchmark foundation

**Deliverables**

- PR CI: format, check, Clippy, unit/integration, docs links.
- Feature compile matrix.
- Deterministic CSV fixture generator với seed/hash.
- Criterion/harness skeleton và benchmark manifest writer.
- Fault-injection hooks được feature-gate cho test.
- CI artifact policy cho raw samples, profiles và crash logs.

**Acceptance**

- Baseline benchmark chạy lại được từ clean clone.
- Fixture generator tạo 1M/100M/1B-row specifications mà không commit dataset lớn.
- CI không dùng noisy shared runner để auto-block bằng threshold hiệu năng thiếu kiểm soát.

### Phase 0 exit gate — `G0`

- BOM/MSRV ADR accepted.
- Tất cả intended feature profiles compile hoặc unsupported profile được loại rõ.
- Benchmark/fault harness tạo artifact tái lập.
- Durable-format ADR backlog có owner.
- Không còn dependency risk có khả năng buộc đổi toàn bộ core public types.

**Effort dự kiến:** 8–12 engineer-weeks.

## 7. Phase 1 — core runtime và resource safety

### `WP-100` — core types, IDs và error model

**Deliverables**

- Newtypes cho pipeline/run/task/attempt/partition/artifact/checkpoint/generation/fencing IDs.
- Persistent offset/counter dùng `u64`.
- Typed errors với stable English error codes và redacted context.
- `PluginDescriptor`, capability và resource request types.
- Canonical hashing/versioning helpers.

**Acceptance**

- Durable identity không phụ thuộc petgraph `NodeIndex` hoặc process-local address.
- Hash inputs có canonical encoding và golden tests.
- Production input path không `unwrap`/panic.

### `WP-110` — resource budget và memory permits

**Deliverables**

- `ExecutorResourceBudget` và validated low/standard profiles.
- Byte-accounted permit pool.
- `AccountedBatch` ownership/lifetime.
- Admission API cho vcore, memory, temporary disk và plugin budgets.
- Pressure state: normal, throttled, spilling, load-shedding.

**Acceptance**

- Property/concurrency tests chứng minh permits không leak/double-release.
- Cancellation trả permits.
- Không queue hoặc worker pool nào được tạo mà thiếu hard bound.
- Metrics phân biệt configured, reserved, used và peak bytes.

**Performance class:** `P0-HOT`.

### `WP-120` — local batch transport và cancellation

**Deliverables**

- `EdgeTransport` contract.
- Local bounded transport theo count và bytes.
- Cancellation propagation và graceful close.
- Backpressure/slow-consumer tests.
- Channel benchmark so sánh candidate implementations.

**Acceptance**

- Producer dừng khi downstream hết permits.
- Drop sender/receiver không deadlock hoặc leak memory.
- Không message per row; unit truyền là `RecordBatch`/accounted batch.

### `WP-130` — local storage/object-store registry

**Deliverables**

- URI resolution và object-store registry.
- Local allowlisted roots/canonicalization.
- Streaming/range reader abstraction.
- Temp/staging directory manager với quota.
- mmap chỉ cho immutable local source đủ điều kiện.

**Acceptance**

- Path traversal, symlink escape và arbitrary device/proc access bị chặn.
- Large read không buffer toàn file.
- Temp quota exhaustion trả typed error và cleanup an toàn.

### Phase 1 exit gate — `G1`

- Core IDs/hash/error APIs ổn định đủ để persistence dùng.
- Memory/backpressure invariants có tests.
- Low profile chạy bounded stream lớn hơn RAM giả lập mà không tăng RSS theo tổng input.
- Local storage security tests pass.

**Effort dự kiến:** 8–14 engineer-weeks.

## 8. Phase 2 — durable control plane, DAG và history

### `WP-200` — SQLite schema, event store và materializer

**Deliverables**

- Migrations cho entities trong history design.
- Append-only event API và payload versioning.
- Serialized SQLite WAL writer/batched append.
- Materialized run/task/attempt state.
- Replay/rebuild command và unknown-event preservation.

**Acceptance**

- Event append và materialized transition nằm cùng DB transaction.
- Duplicate event/sequence bị reject idempotently.
- Rebuild từ event log cho cùng current state.
- Write rate theo task/partition/artifact, không theo row/batch.

### `WP-210` — pipeline spec và DAG compiler

**Deliverables**

- Declarative pipeline schema và normalized hashing.
- DAG cycle/reference validation.
- Stable `TaskId` mapping vào in-memory petgraph.
- Dependency/durability/resource/capability validation.
- Versioned pipeline registry.

**Acceptance**

- Cycle, missing node, duplicate ID và impossible resource request có actionable error.
- Semantically equivalent normalized spec tạo cùng hash.
- Secret plaintext không nằm trong hash history/log.

### `WP-220` — local scheduler, admission, retry và lease

**Deliverables**

- Task state machine.
- Ready-queue theo dependency completion.
- Resource admission và bounded concurrency.
- Retry/backoff/jitter/timeout/cancellation.
- Local lease, heartbeat và monotonic fencing token.

**Acceptance**

- Independent nodes chạy đồng thời trong budget.
- Stale attempt không transition hoặc commit thắng generation mới.
- Cancellation/restart giữ state hợp lệ.
- Deterministic scheduler tests dùng controlled clock.

### `WP-230` — history, lineage và operator CLI

**Deliverables**

- Run/task timeline queries.
- Artifact/checkpoint/source lineage queries.
- CLI cho history, resume inspection và reconciliation dry-run.
- Structured metrics cho event/checkpoint/lease latency.

**Acceptance**

- `AC-010` có integration tests trên seeded run.
- Output không lộ secret/raw sensitive values.
- Timeline ordered theo durable sequence, không theo wall-clock đơn thuần.

### Phase 2 exit gate — `G2`

- DAG/state/retry/lease tests pass.
- Controller restart có thể replay current state.
- History và lineage queryable qua CLI/API nội bộ.
- Chưa cần ETL data path để chứng minh state machine correctness.

**Effort dự kiến:** 16–24 engineer-weeks.

## 9. Phase 3 — resilient CSV ingestion kernel

### `WP-300` — immutable source snapshot

**Deliverables**

- Local source fingerprint/snapshot strategy.
- Copy/reflink/hard-link policy theo filesystem capability.
- Strong hash trong streaming read path khi khả thi.
- Source snapshot event và resume identity.

**Acceptance**

- Mutable source thay đổi làm resume contract mismatch.
- Snapshot lớn không buffer toàn file.
- Hash/size/mtime/path assumptions có platform tests/documentation.

### `WP-310` — bounded sampling và discovery

**Deliverables**

- Seekable multi-window và non-seekable bounded-prefix sampler.
- Encoding decision: explicit, BOM, UTF-8, UTF-16, legacy candidates.
- Dialect scoring.
- Header-depth/profile scoring.
- Audited decision/evidence/fallback events.

**Acceptance**

- UTF-8, UTF-16LE/BE, Shift-JIS và allowlisted single-byte fixtures pass.
- Ambiguous case tuân thủ strict/best-effort/raw-only policy.
- Sampling memory bị chặn bởi config.
- Không silent fallback.

### `WP-320` — record framing và safe partition planning

**Deliverables**

- Quote/escape-aware record scanner.
- Tentative/owned byte ranges và boundary overlap contract.
- UTF-8/UTF-16/Shift-JIS-safe split logic.
- Decoder anchor/replay metadata.
- No-gap/no-overlap partition manifest.

**Acceptance**

- Property tests cắt input tại mọi chunk/byte boundary đại diện.
- Multiline quote, CR/LF/CRLF, escaped quote và truncated record pass policy.
- Union của owned ranges bằng logical records đúng một lần.

**Performance class:** `P0-HOT`.

### `WP-330` — streaming decoder, CSV parser và Arrow builders

**Deliverables**

- UTF-8 fast path không intermediate `String` per row.
- Incremental `encoding_rs` path.
- CSV core parser vào column-oriented Arrow builders.
- Adaptive batch flush theo bytes/rows/pressure/segment.
- `ResumableBatch` progress watermark.

**Acceptance**

- Không per-row task/message/Serde/regex/object map.
- Parser memory ổn định qua 1M và 100M row runs.
- Candidate checkpoint chỉ tại decoder/record-safe state.
- Baseline profile báo cycles/byte hoặc CPU time/byte và allocations.

**Performance class:** `P0-HOT`.

### `WP-340` — header/schema drift và Bronze model

**Deliverables**

- No/single/multi-row header profiles.
- Deterministic duplicate/empty-column normalization.
- Repeated-header detection.
- Changed-header/source-segment lifecycle.
- Bronze schema/provenance và versioned Silver type inference.

**Acceptance**

- Repeated identical header không thành data row.
- Changed header đóng segment và tạo schema/profile version mới.
- Incompatible types không bị lossily ép âm thầm.
- No-header case tạo stable generated column names và warning.

### `WP-350` — validation, quarantine và reject artifacts

**Deliverables**

- Error policies và thresholds.
- Batched reject stream/schema.
- Reject Parquet writer hook.
- Aggregate reason histogram/event.

**Acceptance**

- Invalid rows land hoặc quarantine đúng policy.
- Không ghi một DB row/event cho mỗi reject.
- Raw excerpt/value bị bound và redaction policy rõ.
- `AC-005` pass.

### Phase 3 exit gate — `G3`

- CSV fixture matrix pass cho encoding/header/dialect/drift/corruption.
- Parser/boundary property tests và fuzz smoke pass.
- 100M-row dry ingestion thành Arrow stream với bounded memory.
- Benchmark baseline tồn tại cho UTF-8, UTF-16 và Shift-JIS paths.

**Effort dự kiến:** 24–36 engineer-weeks.

## 10. Phase 4 — durable Parquet, exact resume và local ingestion alpha

### `WP-400` — local Parquet sink transaction

**Deliverables**

- `LakehouseSink`/`SinkTransaction` implementation.
- Parallel bounded partition writers.
- Configurable codec, row-group và part sizing.
- Footer close, flush/fsync, deterministic immutable rename.
- Part statistics/hash/lineage metadata.

**Acceptance**

- Reader mở được mọi committed part.
- Partial/temp file không visible qua manifest.
- Writer memory/concurrency nằm trong permits.
- Output correctness hash/row count được so với input policy.

### `WP-410` — artifact intent, checkpoint và resume contract

**Deliverables**

- Artifact intent/state persistence.
- Canonical resume contract và mismatch diagnostics.
- Partition checkpoint store.
- Commit transaction gắn artifact, checkpoint, events và watermark.
- Fencing-token validation.

**Acceptance**

- Checkpoint không thể commit nếu artifact chưa committed.
- Duplicate logical key cùng content idempotent; khác content bị báo corruption/nondeterminism.
- Crash trước/sau mọi boundary cho kết quả không gap/duplicate visible records.

### `WP-420` — dataset manifest và reconciliation

**Deliverables**

- Versioned manifest generation.
- Atomic local publication.
- Reconciler cho intent/temp/orphan/committed-not-published states.
- Mark-and-sweep roots từ manifest/checkpoint.
- Dry-run và audited cleanup.

**Acceptance**

- Toàn bộ crash matrix trong history document được tự động hóa.
- Reader chỉ thấy committed generation.
- Reconcile lặp lại idempotent.
- GC không xóa artifact đang được retained manifest/checkpoint tham chiếu.

### `WP-430` — local CSV → Bronze Parquet vertical slice

**Deliverables**

- CLI chạy pipeline thật.
- Snapshot → discovery → partition → parse → quarantine → Parquet → manifest.
- History/timeline/lineage/resume CLI.
- Runtime low/standard profiles và metrics.

**Acceptance**

- `AC-002`, `AC-003`, `AC-004`, `AC-005`, `AC-009`, `AC-010` pass trong local scope.
- Kill/restart chỉ reprocess uncommitted ranges.
- Source/config/plugin change từ chối checkpoint cũ.
- Low node 2–4 core, 4–8 GiB xử lý input lớn hơn RAM mà không OOM.

### `WP-440` — 1B-row ingestion qualification

**Deliverables**

- Deterministic 1B-row dataset specification/hash.
- Runs cho narrow/wide, UTF-8 và ít nhất một transcoding path.
- Peak RSS slope, throughput stage metrics, output/reject hashes.
- Crash/restart run trên large dataset.
- Benchmark manifest và bottleneck analysis.

**Acceptance**

- `AC-001` pass trong configured memory envelope.
- RSS đạt steady-state, không tăng tuyến tính theo rows.
- Mọi record land/quarantine đúng một lần trong visible manifest.
- Không quảng bá absolute throughput ngoài measured environment.

### Phase 4 exit gate — `G4` / Release `R1`

- Local ingestion alpha có thể cài và chạy từ clean environment.
- Crash matrix, 1B-row, low-memory và malformed-input gates pass.
- User xem được history/resume/lineage.
- Known limitations được ghi rõ; chưa gọi đây là full product MVP nếu SQL chưa hoàn tất.

**Effort dự kiến:** 16–24 engineer-weeks.

## 11. Phase 5 — DataFusion compute và product MVP

### `WP-500` — DataFusion session và catalog integration

**Deliverables**

- Session factory theo resource envelope.
- Dataset manifest `TableProvider`.
- Projection/filter/limit/partition pushdown capability reporting.
- DataFusion tracked memory pool và spill directory quota.
- Query/operator metrics mapping.

**Acceptance**

- Không nested oversubscription giữa DAG và DataFusion partitions.
- Query lớn hơn memory spill hoặc fail controlled, không OOM.
- Schema versions reconcile theo documented policy.

### `WP-510` — SQL task lifecycle

**Deliverables**

- SQL task compile/plan/execute trong business DAG.
- Input artifact/cache key và transform-code hash.
- Durable SQL output qua sink protocol.
- Cancellation/deadline/retry semantics.

**Acceptance**

- CSV → Bronze → SQL transform → Silver integration pass.
- Retry deterministic không duplicate visible output.
- Nondeterministic task được khai báo và không cache mặc định.

### `WP-520` — workload admission và QoS

**Deliverables**

- Interactive, batch và maintenance resource classes.
- Weighted admission/reserved capacity.
- Per-run/query/tenant vcore, memory, temp disk và deadline limits.
- Load shedding với typed HTTP/gRPC status.

**Acceptance**

- Batch 1B-row workload không làm interactive health/query path starve.
- Hard quota không bị priority bypass.
- Overload giảm throughput có kiểm soát.

### `WP-530` — operator API/CLI completion

**Deliverables**

- Pipeline submit/status/cancel/resume/backfill commands.
- Dataset generation/schema/history inspection.
- Health/readiness/metrics endpoints cho local role.
- Stable machine-readable error codes.

**Acceptance**

- Tất cả local `FR-ORCH-*` và `FR-HIST-*` có traceable test hoặc explicit deferral.
- CLI/config examples khớp implementation thực tế.

### Phase 5 exit gate — `G5` / Release `R2`

- Definition of Done cho MVP trong `docs/07-project-structure-delivery.md` được thỏa.
- Local CSV → Bronze → SQL → Silver chạy, resume và audit được.
- DataFusion memory/spill, concurrent DAG và low-node gates pass.
- Release binary tái lập từ locked dependencies.

**Effort dự kiến:** 20–32 engineer-weeks.

## 12. Phase 6 — serving, cloud và plugin runtimes

Ba lane trong phase này có thể chạy song song sau `G5`, nhưng cùng dùng admission, auth và streaming contracts.

### Lane A — serving

#### `WP-600` — Flight SQL read-only MVP

**Deliverables**

- Statement, schema/info/metadata, prepared statement và `DoGet` flows.
- Signed short-lived tickets bind tenant/subject/partition.
- Query cancellation và result stream accounting.
- Initial JDBC/ODBC compatibility suite.

**Acceptance**

- Slow client không làm RSS tăng không giới hạn.
- Ticket expiry/replay/cross-tenant tests pass.
- Advertised Flight SQL capability phản ánh đúng implementation.

#### `WP-610` — Axum REST/WebSocket Arrow serving

**Deliverables**

- Query lifecycle REST endpoints.
- Arrow IPC stream response.
- WebSocket schema/batch/progress/end framing.
- Authentication, authorization, CORS và quotas.

**Acceptance**

- Không dùng row JSON cho tabular results.
- Disconnect/cancel giải phóng permits/query state.
- Input/body/result limits và path/auth tests pass.

### Lane B — cloud storage

#### `WP-620` — S3/R2 object-store support

**Deliverables**

- S3-compatible configuration/secret resolution.
- Range/vectored read và bounded multipart write.
- Retry/backoff/rate-limit metrics.
- Immutable part objects, manifest generation và conditional pointer CAS.

**Acceptance**

- Không giả định atomic rename hoặc multipart ETag là content hash.
- Timeout, partial multipart, CAS conflict và retry fault tests pass.
- Request amplification/small-file behavior có benchmark.

### Lane C — extension runtimes

#### `WP-630` — plugin registry và capability placement

**Deliverables**

- Registry cho source/sink/transform/auth/catalog descriptors.
- API version/hash/signature/capability validation.
- Built-in registration và feature gating.
- Executor capability advertisement contract.

**Acceptance**

- Duplicate/incompatible/unsigned theo policy bị reject.
- Không load arbitrary Rust `dyn Trait` qua unstable dynamic ABI.

#### `WP-640` — Wasm Component runtime

**Deliverables**

- Versioned WIT host-handle contract.
- Fuel, epoch, memory/table/instance và host-call limits.
- Deny-by-default filesystem/network.
- Compiled component cache có quota.

**Acceptance**

- Infinite loop, memory growth, oversized host call và capability escape tests pass.
- Documentation không tuyên bố zero-copy qua linear memory.

#### `WP-650` — trusted Python/PyO3 profile

**Deliverables**

- PyArrow C Data/C Stream bridge.
- Streaming `RecordBatchReader` contract.
- Model cache/quota/version hash và BLAS/OpenMP thread caps.
- Trusted in-process và subprocess design decision.

**Acceptance**

- Không bắt buộc collect whole input thành table.
- Buffer lifetime/null/type tests pass.
- Python crash/isolation behavior được test và document.
- Minimal local binary không chứa Python dependency.

### Phase 6 exit gate — `G6` / Release `R3`

- Flight SQL và REST/WS backpressure/security tests pass.
- S3/R2 failure injection pass.
- Wasm/Python feature profiles compile và chạy compatibility suites riêng.
- Plugin version/capability nằm trong resume/placement contract.

**Effort dự kiến:** 28–44 engineer-weeks; ba lane có thể song song.

## 13. Phase 7 — distributed execution

### `WP-700` — versioned control protocol

**Deliverables**

- Protobuf cho registration, heartbeat, lease, progress, commit intent và cancellation.
- Protocol/API version negotiation.
- TLS/mTLS và per-RPC authentication hooks.
- Payload/depth/rate limits.

**Acceptance**

- Protobuf field numbers không reuse.
- Unknown fields/version handling có compatibility tests.
- Control message không chứa tabular payload.

### `WP-710` — controller/executor lifecycle

**Deliverables**

- Executor registration/capacity/capability advertisement.
- Heartbeat, draining, lease expiry và fencing.
- Placement/admission theo vcore/RAM/temp disk/locality.
- Reconnect/controller restart behavior.

**Acceptance**

- Executor mất kết nối không làm committed work chạy lại visible.
- Stale executor commit bị từ chối.
- Controller event rate vẫn theo partition/artifact.

### `WP-720` — distributed CSV range ingest

**Deliverables**

- Durable source-range manifest.
- Executor pull/work stealing.
- Direct source read và sink write.
- Bundling small files và partition skew metrics.
- Deterministic speculative attempt policy.

**Acceptance**

- `AC-006` cùng pipeline definition chạy local/distributed.
- Controller nhận metadata, không nhận `RecordBatch` bytes.
- Node loss chỉ reprocess uncommitted owned range.

### `WP-730` — Flight remote edge

**Deliverables**

- Ticketed Flight `DoExchange`/`DoGet` transport.
- Application byte-credit flow control.
- Auth, cancellation, idle/total timeout.
- IPC/compression negotiation trong compatible Arrow universe.

**Acceptance**

- Slow/disconnected receiver propagates backpressure.
- In-flight decoded buffers nằm trong permits.
- Network path được mô tả minimal-copy, không false zero-copy.

### `WP-740` — Ballista adapter

**Deliverables**

- Remote DataFusion query session adapter.
- Business-task resource envelope mapping.
- Custom source/catalog availability trên executors.
- Shuffle/stage metrics và failure mapping vào task history.

**Acceptance**

- Distributed query không tạo scheduler lồng oversubscription.
- Shuffle failure/cancellation/retry có integration tests.
- Ballista không bị dùng để ép non-query task.

### `WP-750` — calibration, placement và skew control

**Deliverables**

- Node calibration fingerprint/results.
- Capacity-aware partition sizing/placement.
- p50/p95/p99 partition/skew metrics.
- Hot-partition split/salting strategy cho supported workloads.

**Acceptance**

- Calibration invalidates khi relevant binary/codec/storage fingerprint đổi.
- Placement không cấp vượt hard resource envelope.

### `WP-760` — distributed qualification

**Deliverables**

- 1/2/4/8-node benchmark matrix.
- Executor/controller/storage/network failure matrix.
- Per-node/aggregate throughput, controller load và scaling efficiency.
- Capacity proof trước bất kỳ 100 GB/phút hoặc TB/phút claim nào.

**Acceptance**

- Non-shuffle target: 2 nodes ≥85%, 4 nodes ≥80%, 8 nodes ≥70% efficiency, hoặc có documented non-code bottleneck/risk decision.
- Controller không nhận tabular bytes theo traffic evidence.
- No loss/duplicate sau executor/controller/storage failures.

### Phase 7 exit gate — `G7` / Release `R4`

- Distributed ingest và query có cùng history/resume/audit semantics với local mode.
- TLS/auth/resource quotas và rolling compatibility tests pass.
- Scaling result có benchmark manifest; marketing claim không vượt evidence.

**Effort dự kiến:** 30–50 engineer-weeks.

## 14. Phase 8 — production hardening, lakehouse và HA

### `WP-800` — production release hardening

- Reproducible build, SBOM, signed artifacts và dependency/license checks.
- Supported platform/MSRV/version policy.
- Upgrade/rollback/migration procedures.
- Security response contacts và private reporting.
- Long-duration soak, disk-full, degraded storage và chaos suites.
- Operational runbooks, capacity planning và backup/restore.

### `WP-810` — snapshot lakehouse catalog

- Spike/ADR chọn Iceberg, Delta hoặc catalog tương đương.
- Snapshot isolation, schema/partition evolution và vacuum.
- Compatibility/migration từ manifested local Parquet.
- Commit conflict/branch/tag/time-travel semantics.

Không gọi MVP manifested dataset là ACID lakehouse trước khi package này hoàn tất.

### `WP-820` — HA metadata/control plane

- PostgreSQL/external durable store hoặc embedded Raft ADR.
- Leader election/fencing và idempotent command/event application.
- Failover/reconnect/clock-skew/network-partition tests.
- Backup, restore và schema migration.

### `WP-830` — compaction, statistics và lifecycle

- Small-file compaction.
- Statistics/index maintenance.
- Retention/legal hold/mark-and-sweep GC.
- Tiering và storage cost controls.

### `WP-840` — DuckDB optional integration

- Arrow C Data Interface compatibility spike.
- In-process ABI/package risk decision.
- Out-of-process Flight/IPC fallback.
- Feature-isolated catalog/query plugin.

### Phase 8 exit gates — `R5` và `R6`

`R5` yêu cầu production hardening và release/security/operations gates. `R6` được phát hành theo từng capability độc lập; HA, lakehouse catalog và DuckDB không cần bị ép vào cùng một release.

**Effort dự kiến:** 30–60 engineer-weeks tùy backend/catalog được chọn.

## 15. Release gate matrix

| Gate | Correctness | Resource/HPC | Security | Operations |
|---|---|---|---|---|
| `G0` | Contracts/BOM coherent | Baseline harness | Dependency provenance | Clean-clone CI |
| `G1` | Typed IDs/errors | Permit/backpressure tests | Local path policy | Metrics bootstrap |
| `G2` | Event replay, DAG, fencing | Metadata rate bounded | Secret redaction | History/CLI |
| `G3` | CSV fixture/property/fuzz | 100M bounded stream | Parser limits | Discovery audit |
| `G4` | Crash matrix/exact resume | 1B rows, low-node RSS | Raw/reject controls | Reconcile/runbook |
| `G5` | E2E Bronze→Silver | DataFusion spill/QoS | Local auth baseline | Reproducible MVP |
| `G6` | Plugin/storage compatibility | Slow clients/cloud benchmarks | Tickets/Wasm/Python policy | Cloud/serving telemetry |
| `G7` | Node-loss no gap/duplicate | 2/4/8 scaling | mTLS/per-RPC auth | Cluster drain/recovery |
| `R5` | Soak/upgrade/restore | Published perf envelope | SBOM/advisory/signing | Supported release |

Không waive correctness/security invariant để đạt throughput gate. Regression có chủ đích cần ADR và documented risk acceptance.

## 16. Test pyramid và cadence

### Mỗi PR

- Format, check, Clippy với warnings denied.
- Unit/integration tests thuộc scope.
- Golden serialization/hash tests nếu contract đổi.
- Docs links/code fences/trailing whitespace.
- Feature compile liên quan.
- Micro/smoke benchmark cho `P0-HOT`/`P1-SCALE` change.

### Nightly

- Full workspace/feature matrix.
- Property tests với seed retention.
- Fuzz smoke cho CSV, IPC, Parquet metadata, Protobuf và tickets.
- Miri/sanitizer jobs cho unsafe/concurrency-sensitive modules khi có.
- 1M/representative benchmark trends.

### Weekly hoặc dedicated runner

- 100M-row encoding/header matrix.
- Peak RSS slope ở ít nhất hai dataset sizes.
- Fault-injection matrix.
- Object-store/network degradation suite khi feature tồn tại.
- Controlled baseline/candidate performance comparison.

### Release candidate

- 1B-row end-to-end.
- Kill tại từng artifact/manifest commit stage.
- Low-node input-larger-than-RAM test.
- BI client compatibility target.
- 1/2/4/8-node qualification cho distributed release.
- Security, dependency, license, SBOM và reproducible-build checks.

Failure artifact phải giữ seed, config, source snapshot/dataset hash, logs đã redact và commit SHA.

## 17. Performance qualification ladder

| Level | Dataset/topology | Mục đích |
|---|---|---|
| `L0` | Microbench, KiB–MiB | Scanner/decoder/parser/kernel cost |
| `L1` | 1M rows | PR smoke và functional profile |
| `L2` | 100M rows | Steady-state memory/throughput |
| `L3` | 1B rows single node | Product acceptance, low/standard profiles |
| `L4` | 2/4/8 nodes | Scaling efficiency và controller isolation |
| `L5` | Hardware capacity envelope | 100 GB/phút/TB-phút claim qualification |

Mỗi level đo riêng raw bytes, decoded bytes, rows, Arrow bytes và committed Parquet bytes. Kết quả không được so chéo các định nghĩa throughput.

Performance budget mặc định:

- Regression credible >5% trên critical throughput/latency: investigate và block nếu không có ADR.
- Peak RSS regression >10%: block nếu không có resource justification.
- Bất kỳ unbounded memory/concurrency hoặc correctness regression: hard fail, không có performance waiver.

## 18. Requirements traceability tới work packages

| Requirement/acceptance | Work packages chính | Evidence cuối |
|---|---|---|
| `FR-ORCH-*` | 210, 220, 230, 530, 700–710 | DAG/state/retry/lease/API tests |
| `FR-HIST-*` | 200, 230, 300, 410–420 | Replay/crash/reconcile/lineage |
| `FR-CSV-*` | 300–350, 430 | Fixture/property/fuzz/E2E |
| `FR-COMP-*` | 110–120, 500–520, 730–740 | Memory/spill/Flight/distributed tests |
| `FR-STOR-*` | 130, 400–420, 620, 810 | Commit/CAS/schema/catalog tests |
| `FR-PLUG-*` | 100, 630–650, 710 | Registry/capability/security tests |
| `FR-SERV-*` | 520, 600–610 | Compatibility/backpressure/auth tests |
| `NFR-COR-*` | 200, 320, 340–350, 410–440 | Golden/property/crash/1B-row |
| `NFR-RES-*` | 110–130, 330, 400, 500, 520 | RSS slope/low-node/quota tests |
| `NFR-PERF-*` | 030, 120, 320–330, 400, 440, 750–760 | Benchmark manifests/scale evidence |
| `NFR-AVL-*` | 200–230, 410–420, 710–760, 820 | Replay/node-loss/failover tests |
| `NFR-SEC-*` | 130, 520, 600–650, 700, 800 | Auth/path/fuzz/sandbox/SBOM |
| `AC-001` | 110, 330, 400, 440 | 1B-row bounded-RSS report |
| `AC-002` | 410, 420, 440 | Full crash matrix |
| `AC-003` | 200, 300, 410 | Resume-contract mismatch tests |
| `AC-004` | 310–340, 430 | Encoding/header/drift fixtures |
| `AC-005` | 350, 430 | Queryable reject artifacts |
| `AC-006` | 210, 700–720 | Same-spec local/distributed E2E |
| `AC-007` | 720, 750, 760 | Traffic proof và scale matrix |
| `AC-008` | 110, 600–610 | Slow-client RSS/backpressure |
| `AC-009` | 110, 130, 330, 400, 440 | Low-node large-input test |
| `AC-010` | 200, 230, 430, 530 | CLI/API timeline/lineage test |

Mỗi issue/PR phải link ít nhất một requirement hoặc nêu rõ `infrastructure-only`.

### Invariant enforcement map

| Invariant | Enforcement owner | Verification bắt buộc |
|---|---|---|
| `INV-001` checkpoint sau artifact | 410–420 | Commit-state constraints và crash matrix |
| `INV-002` exact resume contract | 300, 410 | Contract mismatch/golden hash tests |
| `INV-003` manifest chỉ trỏ object hoàn chỉnh | 400, 420, 620 | Partial-write/CAS/reconcile tests |
| `INV-004` mutable path không là source identity | 300 | Source mutation/resume rejection |
| `INV-005` không buffer toàn input/result | 110–120, 330, 400, 500, 600–610, 730 | RSS slope và slow-consumer tests |
| `INV-006` controller không nằm trên data path | 720, 730, 760 | Controller/network traffic evidence |
| `INV-007` không persist petgraph `NodeIndex` | 100, 210 | Serialization/schema review tests |
| `INV-008` zero-copy claim đúng boundary | 120, 330, 600, 640–650, 730 | Copy/allocation evidence và docs review |
| `INV-009` không event/DB row per record | 200, 350, 760 | Metadata event-rate benchmark |
| `INV-010` coherent Arrow universe | 010 | Feature-specific `cargo tree` gate |

## 19. Parallel work lanes và ownership

Với team từ bốn người trở lên, chia ownership ổn định:

| Lane | Ownership | Có thể song song sau |
|---|---|---|
| Core/runtime | IDs, resource, transport, config | `G0` |
| Control plane | Event store, DAG, scheduler, history | Core IDs ổn định |
| Ingestion | Snapshot, discovery, framing, parser, schema | Core/resource contracts |
| Storage/recovery | Parquet, artifact, checkpoint, manifest | Core + event schema |
| Compute/serving | DataFusion, QoS, Flight, REST/WS | `G4` |
| Extensions/cloud | S3/R2, registry, Wasm, Python | `G5` contracts |
| Distributed | Control protocol, placement, Ballista | `G5` + object-store commit |
| Verification | Fixtures, fuzz, fault, benchmark, release | Từ `G0`, xuyên suốt |

Quy tắc phối hợp:

- Một maintainer sở hữu durable contract; nhiều contributor có thể triển khai consumers sau khi contract merge.
- Hai agent không sửa cùng hot-path file trong một thời điểm nếu không có coordinator.
- Verification lane viết independent oracle/failure tests, không chỉ test theo implementation details.
- Optimization PR đứng sau correctness baseline của cùng path.

## 20. Team capacity và estimate

### Effort tới từng release

| Release | Cumulative engineering effort | Ghi chú |
|---|---:|---|
| `R0` | 8–12 engineer-weeks | Foundation/spikes |
| `R1` | 72–110 engineer-weeks | Local ingestion alpha |
| `R2` | 92–142 engineer-weeks | Product MVP có SQL |
| `R3` | 120–186 engineer-weeks | Serving/cloud/plugins |
| `R4` | 150–236 engineer-weeks | Distributed beta |

Estimate không cộng tuyến tính hoàn toàn vì có lane song song, nhưng concurrency bị giới hạn bởi contract/review/benchmark dependencies.

### Calendar planning tham khảo

- 1–2 experienced engineers: ưu tiên `R1`; không nên hứa full distributed platform sớm.
- 4 engineers: `R2` thường cần khoảng 8–13 tháng nếu dành 60–70% capacity cho delivery sau review/operations.
- 6–8 engineers có ownership rõ: có thể đưa `R2` về khoảng 6–10 tháng và `R4` khoảng 10–16 tháng.

Các khoảng này phải được recalibrate sau `G0`, `G3` và `G4` bằng velocity thực. Không lấy estimate làm lý do giảm fault/performance gates.

## 21. Eight-iteration startup plan

Giả định iteration hai tuần và team bốn người. Đây là sequencing khởi đầu; scope được replan theo evidence.

### Iteration 1 — foundation

- `WP-001` project/bootstrap.
- `WP-010` local/BOM compile spike.
- `WP-030` CI và fixture-generator skeleton.
- Draft ADR BOM/event/checkpoint/memory.

### Iteration 2 — contracts

- `WP-100` IDs/errors/hash.
- `WP-110` memory permit contract/test.
- `WP-200` event schema/migrations skeleton.
- DataFusion→Parquet and parser baseline benchmarks.

### Iteration 3 — core execution

- `WP-120` local bounded transport.
- `WP-210` pipeline spec/DAG compiler.
- `WP-300` source snapshot prototype.
- `WP-400` Parquet sink prototype.

### Iteration 4 — durable state

- `WP-200` event store/materializer completion.
- `WP-220` local scheduler/admission.
- `WP-310` encoding/dialect/header discovery.
- Artifact intent/checkpoint schema.

### Iteration 5 — ingestion kernel

- `WP-320` boundary/partition planner.
- `WP-330` UTF-8 streaming parser fast path.
- `WP-340` header/schema versioning.
- Initial fault hooks around Parquet stages.

### Iteration 6 — commit/recovery

- `WP-410` artifact/checkpoint transaction.
- `WP-420` manifest/reconciler first complete path.
- UTF-16/Shift-JIS incremental decode.
- Reject artifact flow.

### Iteration 7 — vertical integration

- `WP-430` CLI end-to-end.
- Resume-contract mismatch and kill-stage suite.
- 1M/100M memory/throughput runs.
- Operator history/lineage CLI.

### Iteration 8 — `R1` hardening checkpoint

- Fix correctness/resource regressions.
- Expand CSV drift/corruption/fuzz corpus.
- Run initial 1B-row qualification.
- Produce `R1` gap report; không release nếu `G4` chưa đạt.

Iteration 8 là checkpoint, không phải deadline bắt buộc cho `R1`. Các failure ở large-data hoặc crash gate được sửa trước khi chuyển sang SQL/serving.

## 22. Definition of Ready và Definition of Done

### Definition of Ready cho work package

- Objective/non-goals rõ.
- Requirements và dependencies link được.
- Durable/public contract đã có ADR hoặc không cần ADR được giải thích.
- Test oracle và representative dataset đã xác định.
- Performance class đã xác định.
- File/module ownership không trùng agent/contributor khác.
- Security/privacy inputs được phân loại.

### Definition of Done cho work package

- Acceptance conditions pass.
- Correctness, failure và cancellation paths có tests tương xứng.
- Resource bounds có code/config/metrics.
- Benchmark evidence đầy đủ nếu `P0-HOT`/`P1-SCALE`/`P2-LATENCY`.
- Docs/config/changelog/ADR cập nhật.
- Không để `todo!`, unbounded default hoặc silent fallback trên production path.
- Reviewer xác minh actual diff và evidence.
- Handoff ghi rõ tests chưa chạy và risks còn lại.

## 23. Risk burn-down theo phase

| Risk | Phải retire trước | Evidence |
|---|---|---|
| Arrow/DataFusion/Ballista conflict | `G0` | Compile tree/matrix |
| Filesystem+SQLite non-atomicity | `G4` | Crash matrix/reconciler |
| CSV ambiguity/safe boundaries | `G3` | Fixtures/property/fuzz |
| 1B-row memory growth | `G4` | RSS slope/1B report |
| DataFusion oversubscription/spill | `G5` | Concurrent workload tests |
| Flight slow-consumer memory | `G6` | Backpressure/RSS test |
| S3/R2 commit conflict | `G6` | Multipart/CAS fault tests |
| Python process/thread behavior | `G6` | Crash/thread-cap tests |
| Wasm capability/resource escape | `G6` | Sandbox adversarial suite |
| Controller data bottleneck | `G7` | Network/controller traffic proof |
| Distributed stale commit | `G7` | Lease/fencing/node-loss tests |
| Unrealistic throughput claims | Mọi release | Benchmark manifest/capacity model |

Nếu risk không retire được tại gate, project phải chọn một trong ba hành động: giảm scope công khai, kéo dài phase để xử lý, hoặc ADR ghi rõ limitation/risk acceptance. Không chuyển risk correctness thành undocumented technical debt.

## 24. Release artifacts bắt buộc

Mỗi release candidate phải có:

- Source/Cargo lock/toolchain identity.
- Changelog và migration/compatibility notes.
- Requirements/ADR delta.
- Test summary và known skipped tests.
- Benchmark report/manifest theo supported profiles.
- Correctness/fault-injection report.
- Security/dependency/license report.
- Supported platforms/features/limitations.
- Operational upgrade/rollback/recovery notes.
- Artifact hashes; signing/SBOM từ `R5` hoặc sớm hơn nếu public distribution.

## 25. Immediate backlog

Thứ tự issue nên mở đầu tiên:

1. `WP-000`: owner decisions cho tên/LICENSE/DCO/security contacts.
2. `WP-001`: Cargo/toolchain/module bootstrap.
3. `WP-010`: coherent local/distributed/Python dependency spikes.
4. `WP-020`: ADR-0001 BOM/MSRV.
5. `WP-020`: ADR-0002 event/materialized state.
6. `WP-020`: ADR-0003 artifact/checkpoint/manifest ordering.
7. `WP-020`: ADR-0004 memory permits/resource budget.
8. `WP-030`: deterministic CSV fixture generator.
9. `WP-030`: benchmark manifest/harness.
10. `WP-100`: durable IDs, canonical hashing và error codes.
11. `WP-110`: byte-accounted memory permit pool.
12. `WP-200`: SQLite migration/event store skeleton.

Không bắt đầu Ballista customization, arbitrary native dynamic plugins hoặc HA consensus trong immediate backlog.

## 26. Quản lý thay đổi kế hoạch

Plan review diễn ra tại mỗi phase gate:

1. So sánh delivered evidence với acceptance.
2. Cập nhật dependency/critical path.
3. Re-estimate còn lại bằng velocity và benchmark thực.
4. Đóng/mở ADR questions.
5. Cập nhật risk register.
6. Chốt scope release kế tiếp.

Thay đổi làm yếu `INV-*`, exact-resume semantics, storage/wire format hoặc accepted performance regression phải qua ADR. Tái sắp xếp work package không làm thay đổi contract có thể cập nhật trực tiếp trong tài liệu này và changelog.
