# Khung sườn kiến trúc cho contributor

## 1. Mục đích và thẩm quyền

Guide này chuyển kiến trúc trong `docs/01-*` tới `docs/08-*` thành các ranh giới code có thể review. Nó trả lời bốn câu hỏi trước khi contributor viết code:

1. Capability thuộc module nào?
2. Module đó được phụ thuộc trực tiếp vào module nào?
3. Invariant nào phải được giữ tại boundary?
4. Test, benchmark hoặc ADR nào phải đi cùng change?

Guide không thay requirement hoặc ADR. Thứ tự thẩm quyền vẫn là:

```text
INV/correctness/security requirements
    → accepted ADR
    → functional/non-functional requirements
    → domain architecture docs
    → development plan
    → guide này
```

Nếu cần phá dependency direction hoặc đổi ownership bên dưới, không tạo import vòng hoặc đặt tạm code vào module thuận tiện nhất. Mở design discussion và ADR khi change thuộc `docs/development/adr-process.md`.

## 2. Kiểu kiến trúc và composition root

FurrumX là modular monolith. Mỗi top-level Rust module là một service boundary trong cùng process, không chỉ là thư mục nhóm file.

```text
Entrypoints/adapters
    main, cli, serving
            │
            ▼
Composition and application orchestration
    runtime, control
            │
            ▼
Domain/application contracts
    dag, history, checkpoint
            │
            ├──────────────┐
            ▼              ▼
Data-plane capabilities   Extension contracts/adapters
    discovery             plugins
    storage                   │
    transport                 │
    compute ◄─────────────────┘
            │
            ▼
Shared kernel
    core, config, error
```

Đây là dependency direction, không phải runtime data flow. Runtime data có thể đi từ source tới sink, nhưng source code của inner module không được import delivery adapter để gọi ngược ra ngoài.

`runtime` là composition root duy nhất được phép biết hầu hết concrete implementations. `control::controller` điều phối metadata. `control::executor` chạy data-plane work. Vì vậy distributed controller không được nhận `RecordBatch`, Arrow IPC frame hoặc raw tabular bytes.

## 3. Quy tắc dependency

### 3.1 Allowed direct dependencies

Danh sách này là allowlist mặc định. Dependency ngoài danh sách cần được giải thích trong PR; dependency tạo cycle hoặc đảo layer cần design review.

| Consumer | Có thể import trực tiếp | Không được import trực tiếp |
|---|---|---|
| `error` | Standard library, error-formatting dependency đã duyệt | Mọi sibling module |
| `core` | `error`; feature-neutral utility dependency đã duyệt | `dag`, `history`, `checkpoint`, data/adapter modules |
| `config` | `core`, `error` | Concrete storage, network, scheduler hoặc plugin runtime |
| `dag` | `core`, `config`, `error` | `compute`, `storage`, `serving`; DataFusion physical DAG types trong durable model |
| `history` | `core`, `error` | `serving`, `cli`, concrete data-plane adapter |
| `checkpoint` | `core`, `error` | Concrete filesystem/object store, `serving`, `cli` |
| `discovery` | `core`, `config`, `error` | Event database, manifest publisher, controller RPC |
| `storage` | `core`, `config`, `error` | `serving`, `cli`, scheduler; checkpoint advancement |
| `transport` | `core`, `config`, `error`, storage ports cho durable edge | `serving`, business scheduler, controller implementation |
| `plugins` | `core`, `config`, `error`, `discovery`, storage ports; feature-gated engine crates | `serving`, `cli`, controller implementation |
| `compute` | `core`, `config`, `error`, `plugins`, storage/catalog ports, `transport` | `serving`, `cli`, controller implementation |
| `control` | Các contract/application/data-plane module cần cho use case | `cli`, `serving`; không đưa tabular bytes vào controller path |
| `serving` | `core`, `config`, `error`, application facade trong `control`, query facade trong `compute` | Concrete filesystem/SQLite/Parquet internals |
| `cli` | `config`, `error`, application facade do `runtime` cung cấp | Concrete database, object store, parser, writer |
| `runtime` | Mọi module cần để validate config và wire concrete adapters | Business logic mới chỉ tồn tại trong bootstrap code |
| `main` | `cli`, `runtime` | Mọi domain/data-plane implementation |

`use crate::...` chỉ là một dạng dependency. Gọi qua re-export, fully-qualified path, callback, macro hoặc concrete type vẫn được tính là dependency và phải tuân thủ cùng quy tắc.

### 3.2 Quy tắc phá cycle

Khi hai module cần nhau, di chuyển phần giao nhau về đúng contract owner thay vì tạo cycle:

- Durable identity, `ArtifactRef`, `EdgeDescriptor`, capability và resource value types thuộc `core`.
- Port trait thuộc module sở hữu semantics; adapter implementation thuộc module sở hữu technology.
- Orchestration dùng từ ba port trở lên thuộc `control` hoặc `runtime`, không thuộc một adapter.
- DTO dành riêng cho HTTP, Flight hoặc gRPC ở adapter boundary; không đưa DTO đó vào durable domain model.
- Không đặt `NodeIndex`, DataFusion execution-plan node, SQLite row type hoặc Protobuf-generated type trong durable public model.

Ví dụ: checkpoint cần tham chiếu artifact bằng `ArtifactId`/`ArtifactRef` trong `core`; nó không import local Parquet writer. Commit coordinator nhận storage receipt rồi dùng một metadata unit-of-work để atomically ghi artifact state, checkpoint và events. Owner/format của unit-of-work phải được chốt trong ADR; không mô phỏng transaction bằng ba lời gọi độc lập.

## 4. Contract của từng scaffold module

### `core`

Sở hữu shared kernel ổn định:

- Newtypes cho pipeline, run, task, attempt, partition, artifact, checkpoint, generation và fencing token.
- Persistent offsets/counters dùng `u64` và field name có units.
- Artifact/ticket/edge value types không phụ thuộc transport implementation.
- Capability, resource request/budget, cancellation và canonical hashing primitives.
- Protocol-agnostic subject/tenant identity nếu auth providers cần chia sẻ.

Không sở hữu I/O, database schema, retry loop, parser, network middleware hoặc engine session. Feature-neutral phần của `core` phải compile với `--no-default-features`; Arrow-specific batch wrappers được gate bằng feature phù hợp.

### `config`

Sở hữu parsed configuration DTO, validation và resolved runtime profile. Secret value không được tồn tại trong `Debug`, history hoặc normalized config hash; chỉ secret reference/version identity được truyền vào contract.

Config parsing không khởi tạo database, object store, executor hoặc plugin. Việc wire concrete implementation thuộc `runtime`.

### `dag`

Sở hữu business DAG:

- Stable `TaskId`, task/dependency spec và definition hash.
- Cycle/reference/durability/resource/capability validation.
- In-memory graph compilation, admission policy, retry/placement decision logic.
- Scheduler ports và deterministic state-machine helpers.

Không sở hữu DataFusion/Ballista physical query DAG. Không persist petgraph `NodeIndex`. Scheduler không truyền tabular payload.

### `history`

Sở hữu append-only event contract, event-store port, materializer, timeline/lineage query model và reconciliation decision model. Event/state transition phải có sequence/version và được transactionally materialize theo history contract.

Không ghi event theo row/batch và không lưu raw rejects trong control database. Concrete filesystem cleanup được gọi qua storage port; history không tự mở/xóa arbitrary path.

Event ledger, idempotency, projection và SQLite local contract phải tuân thủ [ADR-0002](../decisions/0002-event-store-materialized-state.md).

### `checkpoint`

Sở hữu `ResumeContract`, partition checkpoint, candidate/durable watermark distinction, checkpoint-store port và recovery validation.

Module này không được tự xem một progress watermark là durable. API commit checkpoint phải yêu cầu evidence/reference của committed artifact và fencing generation. Việc publish manifest vẫn là bước riêng sau artifact/checkpoint commit.

### `discovery`

Sở hữu bounded sampling và pure/bounded decisions cho encoding, dialect, header, schema và source segment. Mọi fallback có method/evidence/warning; không có silent fallback.

Immutable source acquisition và raw-zone persistence thuộc storage adapter/application flow. Discovery nhận `SourceSnapshot` đã resolve và không ghi control database trực tiếp.

### `storage`

Sở hữu:

- Object-store registry và secure URI/path resolution.
- Raw-zone snapshot mechanics, streaming/range readers và temp quota.
- Parquet writer/reader, artifact intent/immutable rename/object commit mechanics.
- Manifest format/generation/CAS adapter và storage-side reconciliation operations.

Storage không tự advance checkpoint hoặc mark task succeeded. Nó trả typed receipt để commit coordinator kiểm tra. Reader chỉ mở object được committed manifest tham chiếu.

### `transport`

Sở hữu local bounded batch transport, remote Flight edge, durable edge adapter, byte credits, cancellation và ticket validation liên quan tới edge.

Mọi queue có message cap và byte cap; permit đi cùng buffer/batch tới khi downstream drop. Network path được mô tả là minimal-copy, không phải zero-copy.

### `plugins`

Sở hữu object-safe plugin contracts, descriptor/registry/capability validation và built-in adapters:

- `plugins::datasource` và `plugins::sink` định nghĩa extension ports.
- `plugins::csv` chỉ sở hữu format-specific detect/frame/decode/provider behavior.
- `plugins::python` và `plugins::wasm` là feature-gated runtime adapters với trust/resource limits.

Source snapshot, leases, event history, checkpoint, manifest commit, quarantine retention và resource admission vẫn thuộc platform core/application flow; không dồn chúng vào `CsvPlugin`.

### `compute`

Sở hữu DataFusion session factory, manifest-backed catalog/provider integration, memory/spill accounting, query planning/execution metrics và feature-gated Ballista adapter.

Compute nhận resource envelope từ application layer; nó không tự tạo unbounded pool hoặc quyết định business-task retry. Ballista chỉ chạy physical query workload, không nhận non-query task lifecycle.

### `control`

Sở hữu application orchestration và controller/executor lifecycle:

- Controller: pipeline/run/task commands, metadata scheduling, registry, lease/heartbeat/fencing và commit authorization.
- Executor: trực tiếp chạy source/compute/plugin/sink use case trong resource envelope.
- Commit coordinator: phối hợp storage receipt với metadata unit-of-work bao trùm artifact state, history và checkpoint.
- Distributed control protocol: versioned metadata messages, không chứa tabular payload.

Đây là nơi phù hợp cho local ingestion application service ban đầu. Chỉ tạo top-level `ingestion` module nếu scope/ownership đã đủ lớn và được design review; không tạo để né dependency rule.

### `serving`

Sở hữu Flight SQL, REST/WebSocket, auth middleware, signed tickets, request/result limits và slow-consumer behavior. Handler gọi application/query facade; không mở SQLite, filesystem hoặc Parquet writer trực tiếp.

Cross-protocol identity/policy types nằm ở shared contract; HTTP/Flight middleware ở `serving`, gRPC middleware ở `control`. Auth provider có thể là plugin nhưng trust decision không do plugin tự cấp.

### `runtime`, `cli` và `main`

`runtime` validate profile, tạo Tokio/CPU pools, storage/event adapters, registries, controller/executor/serving services và graceful shutdown graph. Không đặt business rule chỉ tồn tại trong một role bootstrap.

`cli` parse input, gọi application facade và render redacted output. `main` chỉ parse/bootstrap/exit mapping. CLI không được trở thành service locator gọi thẳng mọi module.

## 5. Standard layout bên trong một module

Không bắt buộc tạo file rỗng, nhưng khi implementation bắt đầu, mỗi boundary nên tách bốn vai trò rõ ràng:

```text
module/
├── mod.rs          # boundary docs và curated re-exports
├── model.rs        # value types/state transitions owned by module
├── ports.rs        # traits mà application cần từ adapter
├── service.rs      # orchestration/policy chỉ của boundary này
└── <adapter>.rs    # SQLite/local/Flight/DataFusion/... implementation
```

Dùng tên cụ thể trong `docs/07-project-structure-delivery.md` thay cho tên generic khi đã có, ví dụ `event_store.rs`, `compiler.rs`, `manifest.rs`, `flight.rs`. Không export toàn bộ implementation bằng wildcard từ `mod.rs`.

Mỗi port phải định nghĩa:

- Input/output type và units.
- Error/cancellation semantics.
- Idempotency và retry behavior.
- Resource/concurrency bounds.
- Version/compatibility nếu durable hoặc remote.
- Observability không chứa secret/raw sensitive data.

## 6. Ba luồng bắt buộc phải giữ nguyên boundary

### 6.1 Local CSV tới Bronze

```text
cli/serving command
    → control application service
    → storage resolves immutable SourceSnapshot
    → discovery creates audited decisions/segments
    → plugins::csv streams resumable batches
    → transport applies byte backpressure
    → storage writes immutable Parquet/reject artifacts
    → control commit coordinator validates fencing
    → atomic metadata transaction records artifact + checkpoint + events
    → storage publishes manifest generation
    → history marks terminal state
```

Không checkpoint trước artifact receipt. Không để `plugins::csv` publish manifest. Không để controller nhận batches trong distributed mode.

### 6.2 SQL/query serving

```text
serving authenticates and authorizes
    → control/compute admission grants resource envelope
    → compute plans and executes through manifest-backed provider
    → transport streams byte-accounted batches
    → serving writes Arrow IPC/Flight and propagates cancellation
```

Handler không `collect()` toàn result. SQL parsing không thay authorization.

### 6.3 Distributed execution

```text
controller: task metadata → lease + fencing token → executor
executor: source bytes → batches → artifact directly to storage
executor: aggregate receipt/progress → controller
controller: validate lease/commit → publish metadata state
```

Control Protobuf không có field chứa `RecordBatch`, Arrow IPC payload hoặc raw file chunk. Remote tabular exchange dùng Flight/artifact.

## 7. Public API và feature gates

- `pub` là compatibility decision. Dùng `pub(crate)` mặc định; chỉ re-export type đã có owner, docs, error semantics và compatibility intent.
- Top-level scaffold namespace hiện không phải lời hứa API ổn định. Trước public release, `lib.rs` phải curate surface thay vì export mặc định mọi implementation module.
- Feature-neutral IDs/config/errors phải compile không default features.
- Arrow/DataFusion/Parquet/local adapters nằm sau `local`.
- Flight SQL, S3, Wasm, Python và distributed code chỉ compile khi feature tương ứng bật.
- Không dùng type từ optional dependency trong API được compile khi feature đó tắt.
- Không lấy transitive dependency làm API dependency; dependency dùng trực tiếp phải khai báo trực tiếp và qua dependency review.
- Mọi supported feature profile phải giữ một Arrow/DataFusion/Ballista major-version universe theo ADR-0001.

## 8. Cách đặt một feature mới

| Feature cần thêm | Contract owner | Adapter/implementation | Application wiring |
|---|---|---|---|
| File format mới | `plugins` + reusable decision types trong `discovery` | `plugins::<format>` | `control::executor`, `runtime` |
| Object store mới | Storage port hiện có | `storage::registry`/backend adapter | `runtime` |
| Business task mới | `dag` task contract | Plugin/compute/storage adapter phù hợp | `control` |
| Event/history query mới | `history` | Event-store adapter | `control` facade, rồi `cli`/`serving` |
| SQL operator/provider | `compute`/plugin contract | DataFusion adapter | `runtime` registry |
| API endpoint mới | Existing application facade | `serving` handler | `runtime` router |
| Control RPC mới | `control` protocol contract | gRPC adapter | Controller/executor runtime |
| Cross-cutting resource policy | `core` value type + owning domain policy | Adapter metrics/enforcement | `control` admission |

Nếu cùng feature cần sửa từ ba boundary trở lên, chia PR theo contract → adapter → integration. PR đầu tiên không được tạo placeholder contract thiếu semantics chỉ để unblock parallel code.

## 9. Definition of Ready cho contributor

Trước khi code:

- Chỉ ra work package và requirement/invariant liên quan.
- Chọn một owning module và liệt kê direct dependencies mới.
- Mô tả port/receipt nào đi qua boundary; không truyền concrete adapter type ra ngoài.
- Xác định durable/public/wire/storage format có cần ADR không.
- Xác định resource bounds, cancellation và retry/idempotency.
- Chọn test oracle, failure cases và performance class.
- Xác nhận file ownership không trùng contributor khác.

## 10. Definition of Done cho một feature slice

- Module boundary và dependency direction vẫn đúng.
- Contract types/tests được merge cùng hoặc trước implementation consumer.
- Không silent loss/fallback; errors actionable và đã redact.
- Queue/cache/concurrency/temp disk có hard bound.
- Retry, cancellation, stale lease và partial failure được test theo scope.
- Durable change có golden/version/migration/fault evidence phù hợp.
- Hot-path change có benchmark manifest theo performance policy.
- Feature matrix liên quan compile/test; skipped verification được ghi rõ.
- Docs/config/changelog/ADR và requirement traceability đã cập nhật.

## 11. Các quyết định chưa được guide này tự chốt

Các mục sau vẫn cần spike/ADR trước implementation production:

- Artifact/checkpoint/manifest commit API và local fsync guarantees.
- Manifest serialization format.
- Memory permit/accounting API.
- Raw/Bronze/Silver durable schema.
- Shared auth/policy contract nếu vượt protocol middleware.
- Tách top-level `ingestion`, `catalog`, `security` hoặc Cargo crates.

Guide chỉ xác định nơi giữ boundary trong lúc các quyết định đó còn mở; nó không thay thế quyết định durable/public format.
