# ADR-0004: Byte-accounted memory và temporary-disk permits

- Status: Accepted
- Date: 2026-08-05
- Owners: Project owner, runtime/resource maintainers and performance maintainers
- Deciders: Project owner, runtime/resource maintainer, compute maintainer and performance maintainer
- Related requirements: `FR-ORCH-003`, `FR-ORCH-009`, `FR-COMP-003`, `FR-COMP-006`, `FR-SERV-005`, `NFR-RES-001` tới `NFR-RES-005`, `NFR-PERF-001`, `NFR-PERF-002`, `NFR-PERF-007`, `NFR-AVL-002`, `NFR-AVL-003`, `NFR-SEC-006`, `INV-005`, `INV-006`, `AC-001`, `AC-008`, `AC-009`
- Related work packages: `WP-020`, `WP-030`, `WP-100`, `WP-110`, `WP-120`, `WP-220`, `WP-320`, `WP-330`, `WP-350`, `WP-400`, `WP-420`, `WP-500`, `WP-520`, `WP-620`, `WP-730`, `WP-740`
- Related decisions: ADR-0001, ADR-0002 and ADR-0003
- Supersedes: N/A

## Context

FurrumX phải xử lý input lớn hơn RAM nhiều lần trên node 4–8 GiB mà không OOM, đồng thời không hy sinh throughput trên node lớn. Giới hạn số task, số `RecordBatch` hoặc số file không đủ vì một batch/file có thể rộng hơn nhiều lần batch/file khác. DataFusion `MemoryPool` quản lý các operator giữ state lớn, nhưng không theo dõi toàn bộ streaming batches, parser/decoder buffers, network frames, Parquet writers, plugins, metadata queues hoặc allocator/runtime overhead.

Temporary disk cũng là tài nguyên hữu hạn. Spill, staging Parquet, manifest temp files, reject artifacts và recovery leftovers có thể dùng chung filesystem. Nếu chỉ kiểm tra free space trước khi bắt đầu, nhiều writer concurrent vẫn có thể oversubscribe và cùng gặp `ENOSPC`. Sau crash, in-memory permits biến mất nhưng temp files còn tồn tại.

Resource model phải giải quyết đồng thời:

- Admission giữa nhiều task/query/tenant.
- Byte ownership xuyên source → parser → Arrow → transport → compute → writer.
- Peak coexistence khi input và output cùng sống trong transform/transcode/network boundary.
- Spill/staging disk và crash-recovery debt.
- Cancellation, timeout, shutdown và permit leak.
- DataFusion/Arrow/Tokio integration mà không tự viết allocator, spill engine hoặc async semaphore.
- RSS/cgroup pressure do allocations không thể account chính xác bằng domain permits.

## Scope

ADR này quyết định:

- Node capacity, safety headroom, task resource request/envelope và hierarchical ownership.
- Byte measurement, quantization và RAII lease semantics.
- Memory charge khác channel byte-credit như thế nào.
- Queue, Arrow batch, parser/writer, metadata và plugin accounting.
- DataFusion memory/spill integration và double-accounting boundary.
- Temporary-disk reservation, growth, cleanup and restart reconciliation.
- Deadlock prevention, cancellation, resize and pressure/load-shedding behavior.
- Local/distributed ownership, error/metric contract and verification gates.

ADR này không quyết định:

- CPU scheduling implementation hoặc tenant weighted-fair policy chi tiết; thuộc `WP-220`/`WP-520`.
- Custom global allocator, jemalloc/mimalloc adoption hoặc kernel cgroup provisioning.
- Ballista internal memory protocol; executor vẫn phải enforce local envelope around it.
- Durable dataset storage quota/retention billing; temporary disk phải transfer ownership vào storage quota tại artifact commit.
- Exact parser batch/Parquet row-group targets; các giá trị đó phải benchmark theo profile.
- Cluster-wide distributed semaphore; mỗi executor là final authority cho local resources.

## Decision drivers

- Không allocation/write đáng kể trước khi có capacity tương ứng.
- Tổng admitted envelopes không vượt managed node capacity.
- Mỗi allocation/file có đúng một memory/disk charge owner; view/ownership transfer không double-charge.
- Channel byte-credit không bị nhầm với physical memory charge.
- Permit release theo RAII/drop và cancellation-safe.
- Không async wait vô hạn khi đang giữ tài nguyên cần cho progress.
- Resource counters dùng `u64`; conversion sang `usize`/`u32` phải checked.
- Accounting overhead theo buffer/batch/file/task, không theo row/cell.
- DataFusion dùng native memory/spill primitives trong pinned BOM.
- Permit logic không tự viết lock-free queue, custom allocator hoặc filesystem quota engine.
- RSS/disk external pressure phải fail/load-shed trước OS OOM/disk-full.

## Options considered

### Option A — Giới hạn theo task/batch/message count

Ưu điểm:

- Dễ implement.

Nhược điểm:

- Không phản ánh wide rows, compressed/decoded coexistence hoặc multipart part size.
- Cùng count có thể dùng memory/disk chênh lệch hàng chục lần.
- Không đáp ứng `NFR-RES-002`.

Option này bị từ chối. Count cap vẫn bắt buộc để hạn chế metadata/task overhead nhưng không thay byte cap.

### Option B — Chỉ dùng DataFusion `MemoryPool`

Ưu điểm:

- Ít code và tận dụng engine.

Nhược điểm:

- Không cover ingestion, streaming `RecordBatch`, Flight/IPC buffers, Parquet writer, plugin, metadata queue và non-query task.
- DataFusion tài liệu nói rõ pool tập trung vào large stateful consumers, không phải mọi allocation.
- Không quản lý shared temp disk giữa non-DataFusion writers.

Option này bị từ chối làm global policy; DataFusion pool vẫn được dùng bên trong compute envelope.

### Option C — Hierarchical fixed envelope + quantized RAII leases

Control admission reserve một task envelope từ node capacity trước execution. Task chia envelope thành bounded child budgets. Buffers/files giữ movable RAII leases; Tokio semaphore cung cấp async waiting, DataFusion native pools quản lý engine internals, Arrow memory-size APIs cung cấp conservative batch charge.

Ưu điểm:

- Hard upper bound dễ reasoning và test.
- Ownership transfer tự nhiên với Rust move/RAII.
- Không cần custom allocator/semaphore/spill engine.
- Giữ overhead theo batch/file, không theo row.
- Task failure/cancellation trả capacity tự động.

Nhược điểm:

- Fixed envelopes có thể underutilize tài nguyên khi task request quá bảo thủ.
- Quantization/conservative Arrow size có over-accounting.
- Cần progress reserve và lock/acquire order để tránh deadlock.

Option này được chọn cho MVP.

### Option D — Elastic global pool/custom allocator với work stealing

Mọi allocation lấy từ một custom global allocator/token scheduler; task có thể vay/trả động.

Ưu điểm:

- Utilization lý thuyết cao.

Nhược điểm:

- Complexity lớn ở cancellation, fairness, priority inversion, allocator hooks và debugging.
- Dễ double-account với Arrow/DataFusion/Python/Wasm.
- Custom unsafe allocator trái application safety baseline và chưa có benchmark evidence.

Option này bị từ chối. Elastic cross-task borrowing chỉ được xem xét bằng benchmark + superseding ADR sau khi fixed-envelope utilization thực sự là bottleneck.

## Decision

### 1. Capacity hierarchy

Resource hierarchy:

```text
Detected host/cgroup/filesystem capacity
        └── NodeCapacity (managed hard limits + unreachable safety reserve)
                └── admitted TaskResourceEnvelope
                        ├── input/decode/parser memory
                        ├── transport/serving memory
                        ├── DataFusion memory
                        ├── writer/plugin/control memory
                        ├── progress reserve
                        └── temporary-disk budget
```

Feature-neutral domain types dùng explicit units:

```text
MemoryBytes(u64)
TemporaryDiskBytes(u64)
ComputeVcores(u16)
ResourceRequest
TaskResourceEnvelope
ResourceClass
ResourceLeaseId
```

Rules:

- Scheduler/controller có thể prefilter, nhưng executor/node governor là final admission authority.
- Một task chỉ bắt đầu sau khi atomically reserve toàn `TaskResourceEnvelope` từ node pool.
- Sum của live task envelope memory/temp-disk/vcore không vượt node managed limits.
- Child budget sum không vượt task envelope. Normal allocation không vay safety headroom.
- Envelope gắn run/task/attempt/lease identity, deadline và cancellation token; không persist process-local semaphore permit.
- Retry/attempt mới cần envelope mới; stale attempt cancellation trả resources nhưng không thay durable commit semantics.
- Remote controller advertisement chỉ là snapshot. Executor có quyền reject stale placement bằng typed resource error.

MVP không cho cross-task memory loans. Unused bytes trong một task có thể được repartition giữa child categories qua task-local governor nếu progress reserve và task total vẫn giữ nguyên; không trả/vay global capacity giữa chừng.

### 2. Node capacity và safety headroom

Startup xác định effective memory ceiling từ explicit config và các limits khả dụng như cgroup/container/host; chọn giá trị nhỏ nhất đáng tin cậy. Nếu không xác định an toàn, low/local profile phải dùng conservative documented default hoặc yêu cầu config, không giả định toàn physical RAM có thể dùng.

```text
managed_memory_bytes
  = effective_memory_limit_bytes - safety_headroom_bytes
```

Initial automatic profile giữ safety headroom bằng `max(15% effective limit, 512 MiB)` trên supported 4 GiB+ nodes. Không cho cấu hình thấp hơn 10% nếu không có maintainer-approved environment evidence. Đây là initial safety policy, không phải performance claim; `WP-110` phải hiệu chỉnh bằng RSS/allocation benchmarks.

Safety headroom cover allocator fragmentation, Tokio/runtime stacks, SQLite, TLS/network, untracked DataFusion/Arrow metadata, code/native libraries và measurement error. Nó không phải pool để task acquire. Khi headroom bị ăn vào, governor chuyển pressure state thay vì tăng limit.

Temporary disk capacity dùng configured roots. Mỗi root có:

```text
managed_temporary_disk_bytes
filesystem_safety_reserve_bytes
observed_recovery_debt_bytes
```

Initial filesystem reserve là `max(5% filesystem capacity, 2 GiB)` trừ khi dedicated volume policy cung cấp giá trị lớn hơn. Managed temp capacity không được vượt current safe available bytes sau reserve/recovery debt. External writers có thể làm free space thay đổi, nên runtime recheck stat/cgroup/filesystem pressure định kỳ và trước large growth.

### 3. Quantized permit implementation

Không dùng một semaphore permit cho một byte. Concrete local implementation bọc `tokio::sync::Semaphore` bằng quantized checked accounting:

```text
memory quantum:         4 KiB
temporary-disk quantum: 1 MiB
units = ceil(requested_bytes / quantum_bytes)
```

Domain/config counters là `u64`. Trước khi gọi Tokio API, conversion `u64 bytes → u64 units → u32 acquire_many` phải checked. Pool construction cũng validate against `Semaphore::MAX_PERMITS`; invalid config trả typed startup error, không panic.

Quantization overcharges phần lẻ và không bao giờ undercharge. Quantum có thể configurable tại startup nếu power-of-two, bounded và có benchmark evidence; durable semantics không phụ thuộc quantum. Không acquire per row/cell.

`BytePool`/`ByteLease` wrapper phải cung cấp tối thiểu:

```text
try_acquire(bytes)
acquire(bytes, deadline, cancellation)
grow_before_use(additional_bytes)
shrink_to(actual_capacity_bytes)
charged_bytes()
transfer/move ownership without reacquire
close() for shutdown
```

Lease giữ owned RAII permit; `charged_bytes()` luôn trả rounded physical charge `units × quantum_bytes`, không phải requested logical bytes. `shrink_to` làm tròn `actual_capacity_bytes` lên quantum, chỉ trả phần quantum không còn cần và không được giảm dưới allocation/file còn sống. Request zero bytes trả explicit zero-sized lease mà không chạm semaphore; chỉ empty buffer/file được giữ zero lease và mọi growth dương vẫn phải acquire trước. Last drop trả units. `mem::forget`, detached task giữ lease vô hạn, raw `add_permits` hoặc public access tới inner semaphore bị cấm trên production path. Resize pool downward không revoke live leases: stop new admission, đợi reserved xuống target hoặc fail controlled maintenance operation.

ADR-0004 extends ADR-0001 feature mapping: local implementation cần direct Tokio `sync` support. `WP-110` phải bật `tokio` cho local runtime với minimal required features, giữ `--no-default-features` feature-neutral contracts compile được và không kéo Tonic/Ballista. Nếu không đạt feature isolation này, cần superseding BOM ADR.

### 4. Physical charge, reservation và flow credit

Ba khái niệm không được trộn:

1. **Task envelope reservation**: capacity node dành riêng cho task; đo reserved upper bound.
2. **Physical memory/disk charge**: allocation/file thực trong envelope; đo current/peak use.
3. **Flow byte-credit**: giới hạn in-flight bytes của một channel/connection; không cộng lần hai vào physical memory total.

Một buffer trong channel giữ physical `MemoryLease` của allocation và thêm `ChannelByteCredit` để enforce channel cap. Metrics báo riêng `memory_charged_bytes` và `channel_in_flight_bytes`; cộng hai số này thành RSS estimate là sai.

Mọi queue/channel/cache có cả item count cap và byte-credit cap. Count cap bảo vệ task/header/Arc overhead; byte cap bảo vệ payload variance. Producer acquire channel credit trước enqueue và move credit cùng message; consumer/drop/cancellation trả credit. Queue không tự đo lại rồi cấp một physical lease thứ hai.

### 5. Allocation ownership và Arrow batches

Nguyên tắc bắt buộc:

- Acquire/grow lease trước allocation, buffer growth, decode output, compression workspace hoặc write vượt reserved bytes.
- Charge allocated capacity/physical buffer size, không chỉ logical length/row count.
- Khi compressed input và decoded output cùng sống, cả hai được charge đồng thời.
- Projection/slice/`Arc` clone không allocation mới phải share cùng allocation lease; không charge lại từng view.
- Kernel/concat/filter/materialization tạo buffers mới phải acquire output lease trước compute.
- Permit chỉ release khi allocation cuối cùng thực sự drop, không phải khi enqueue/acknowledge.
- API thư viện không cho biết chính xác output trước execution phải chạy dưới bounded maximum-operation reservation lấy trước, rồi reconcile xuống physical capacity thực; không được allocate trước rồi mới thử acquire.

Arrow integration dùng existing `RecordBatch::get_array_memory_size`/`ArrayData` memory-size APIs làm conservative baseline. API này có thể overestimate shared buffers; FurrumX không viết pointer-dedup allocator. `AccountedBatch` giữ một bounded `AllocationOwners` gồm các `Arc<AllocationLease>` để view dùng lại buffer giữ cùng charge. Projection/slice giữ owners cũ dù có thể over-account; batch ghép không cấp buffer mới từ nhiều input hợp nhất owners theo `ResourceLeaseId`, không theo địa chỉ buffer. Số owner bị bound bởi validated schema/input fan-in; vượt bound phải materialize dưới lease mới hoặc fail typed resource error. Khi operation tạo output mới, output có lease riêng; input leases sống tới khi input buffers drop.

```text
AccountedBatch = RecordBatch + bounded AllocationOwners + logical metadata
```

Builder/slab code biết capacity thực phải charge capacity trực tiếp trước growth và chuyển lease vào frozen batch. `get_array_memory_size` là validation/fallback, không được gọi per row hoặc lặp vô ích trên hot path.

### 6. Progress reserve và deadlock prevention

Bounded system vẫn có thể deadlock nếu mọi task giữ input memory rồi chờ output/spill capacity. Mỗi task envelope phải dành một non-borrowable `progress_reserve_bytes` đủ cho ít nhất một supported maximum output batch/flush workspace plus control/cancellation overhead của task profile.

Acquire/order rules:

```text
1. Node task envelope is acquired atomically before execution.
2. Task child physical leases are acquired before allocation/use.
3. Channel credits may wait only while the bounded accounted message exists.
4. No code waits for another node/task envelope while holding one.
5. Disk spill/growth never waits indefinitely while holding all releasable memory.
```

Nếu output lease không có:

1. Flush/release completed input or writer buffers.
2. Reduce adaptive batch/row-group target within configured minimum.
3. Spill through pre-reserved temp-disk envelope where semantics support it.
4. Propagate backpressure/cancel lower-level work.
5. Return typed resource exhaustion before deadlock/OOM.

Không giữ blocking lock qua `.await`. Không acquire nhiều independent pools theo caller-defined order. Cross-pool operation phải dùng one coordinator method với documented order hoặc pre-reserved envelope. Tests dùng timeout chỉ làm oracle; production không “sửa deadlock” bằng retry vô hạn.

Tokio semaphore FIFO fairness được giữ; large `acquire_many` có thể gây head-of-line blocking. Vì vậy large task envelopes được xử lý ở control admission, không xếp vào data-path semaphore queue; per-buffer acquire bị giới hạn bởi supported max batch/part chunk. Không tự viết unfair/lock-free semaphore để né vấn đề này.

### 7. DataFusion integration

Mỗi admitted query/task nhận một DataFusion budget cố định từ child `datafusion_memory_bytes` và `temporary_disk_bytes`:

- Configure `RuntimeEnv` with bounded native `FairSpillPool` for spill-capable mixed operators; wrap/use consumer tracking available in pinned DataFusion for diagnostics.
- `GreedyMemoryPool` chỉ dùng khi workload/operator semantics chứng minh fair spilling không phù hợp.
- `UnboundedMemoryPool` bị cấm trong supported production profiles.
- Configure DataFusion built-in temp path/directory size limit (`RuntimeEnvBuilder`/`DiskManagerBuilder`) inside task-owned spill root; không viết spill engine thứ hai.
- Configure cache limits explicitly; default unbounded/implicit cache không được dựa vào.
- `target_partitions` và concurrent plans phải nằm trong admitted vcore/memory envelope.

Node admission đã reserve `datafusion_memory_bytes` một lần. DataFusion `MemoryReservation` là child internal accounting, không acquire lại global `BytePool`; nếu không sẽ double-charge. Streaming input/output batches ngoài large operator reservations tiếp tục giữ external `AccountedBatch` leases vì DataFusion không track mọi batch/allocation.

DataFusion docs lưu ý memory limit chưa cover mọi case; vì vậy outer headroom, RSS guard và external batch accounting vẫn bắt buộc. Adapter phải export DataFusion reserved/top-consumer/spill metrics cùng FurrumX envelope metrics.

### 8. Temporary-disk lifecycle

Task reserve maximum temp-disk envelope trước execution. Bên trong envelope:

- Create temp/spill/staging file chỉ sau khi có `TempDiskLease` và secure task-scoped path.
- Known maximum consumers như DataFusion query may reserve their assigned disk sub-budget up front and pass it to built-in `DiskManager` limit.
- DataFusion `DiskManager` sub-budget là single charge owner cho spill files của engine; các file đó không acquire thêm `TempDiskLease`. Adapter reconcile native reported/on-disk usage với assigned sub-budget và outer filesystem pressure guard.
- Incremental writers grow lease before next write/flush can exceed reserved bytes; after flush/stat they shrink to actual allocated/logical policy bytes.
- Sparse files account `max(logical_size_bytes, allocated_blocks_bytes when available)` according to adapter safety policy; không dựa vào apparent zero-filled size để undercharge.
- File lease chỉ release sau successful delete, or transfer into durable storage quota/ownership after ADR-0003 physical commit.
- Rename không tự giải phóng physical disk nếu source/destination cùng volume.
- Failed delete giữ recovery debt và metric; không cộng permit về pool trước khi bytes thực sự được reclaim/transfer.

Durable final artifacts không nằm mãi trong temp quota. At boundary `P/M`, storage atomically chuyển ownership accounting từ staging lease sang configured durable storage capacity/retention ledger. Nếu durable capacity không được cấp, artifact commit không được giả vờ giải phóng disk; task phải backpressure/fail trước checkpoint.

### 9. Restart reconciliation và external disk pressure

Permit objects không durable. Trước writable readiness, runtime scan only configured task-scoped temp roots, correlate files với artifact/spill/manifest intents và tính `observed_recovery_debt_bytes` bằng bounded directory pages.

Rules:

- Existing temp bytes bị trừ khỏi managed available capacity trước admission mới.
- Unknown/symlink/path-escape entry fail/alert theo security policy; không follow symlink hoặc recursive-delete arbitrary path.
- Active/recoverable file được reattach logical recovery ownership; orphan file chỉ delete sau lease/grace/reconcile policy.
- Cleanup idempotent; disk permit chỉ trở lại sau verified deletion.
- Incomplete object-store multipart uploads không phải local temp bytes nhưng vẫn cần adapter abort/provider lifecycle metrics.

Nếu filesystem free bytes giảm ngoài accounting:

1. Stop new disk-heavy admission.
2. Flush/cleanup safe temp and evict caches.
3. Reduce concurrency/batch targets.
4. Load-shed/cancel work theo policy nếu pressure tiếp tục.
5. Fail controlled before safety reserve is consumed.

Permits không được quảng bá là protection tuyệt đối trước external processes; filesystem reserve + periodic observation là secondary guard.

### 10. Pressure state và RSS guard

Logical permit compliance là cần nhưng chưa đủ để bound RSS. Node governor quan sát RSS/cgroup memory events, allocator/process metrics khi available và disk free space:

```text
Normal → Throttled → Flushing/Spilling → LoadShedding
                                      ↘ ControlledFailure
```

- `Normal`: admission và configured concurrency.
- `Throttled`: stop new partitions/tasks, reduce producer concurrency/targets.
- `Flushing/Spilling`: request cooperative flush/spill/evict; không phá correctness.
- `LoadShedding`: reject new query/run with retry hint; cancel only theo explicit priority/deadline policy.
- `ControlledFailure`: fail task/process readiness before OS OOM-kill/disk-full corrupts work.

Pressure transitions có hysteresis và minimum dwell time để tránh oscillation. Exact thresholds thuộc calibrated profile/config; critical threshold không được vượt effective hard limit hoặc filesystem safety reserve. Safety headroom không được cấp cho throughput work.

### 11. Metadata, serving, plugin và distributed boundaries

- ADR-0002 metadata writer queue reserves command count + serialized/envelope bytes before enqueue; raw payload/reject rows vẫn bị cấm.
- Flight/REST/WS charge compressed/wire buffer và decoded Arrow/output buffer khi cùng sống; slow client giữ bounded channel credit and memory lease.
- Wasm dùng Wasmtime native store/resource limit/fuel/epoch mechanisms plus outer task envelope; không tự viết Wasm allocator.
- Trusted same-process Python giữ outer envelope nhưng không được claim hard isolation. Hard untrusted limit cần child process/container/cgroup policy; Pandas/table collection chỉ khi request đủ envelope.
- Plugins declare worst-case working-set/temp-disk bounds or are rejected/placed in restricted profile. Plugin không tự tạo pool/cache/thread count ngoài envelope.
- Distributed executor enforce local permits; controller không giữ global buffer permit và không nhận tabular bytes.
- Flight credit là per-connection/executor; control messages chỉ mang bounded numeric grant/usage summaries.

### 12. Error, cancellation và observability contract

Stable error families:

```text
RESOURCE_REQUEST_EXCEEDS_CAPACITY
RESOURCE_ACQUIRE_TIMEOUT
MEMORY_BUDGET_EXHAUSTED
TEMP_DISK_BUDGET_EXHAUSTED
RESOURCE_ACCOUNTING_VIOLATION
TEMP_DISK_RECOVERY_DEBT
RESOURCE_POOL_CLOSED
```

Errors include resource class, requested/limit/used/available bytes and retryability but no secret/path outside redacted storage root identity. Cancellation while waiting loses queue position and acquires nothing; cancellation after acquire drops/returns lease through structured task ownership. Shutdown closes pools, rejects new acquire and waits bounded time for live leases before reporting leaks.

Metrics tối thiểu:

```text
node/task envelope requested, admitted and rejected bytes
memory/disk reserved, charged, available and peak bytes
safety headroom and RSS delta versus charged bytes
queue item count, channel credit bytes and backpressure time
permit acquire wait p50/p95/p99 and timeout count
DataFusion reserved/top consumers/spill bytes/files/time
temp recovery debt, cleanup bytes/failures and filesystem free bytes
pressure-state duration/transitions and load-shed count
live lease count/oldest age by bounded consumer label
```

Consumer labels đến từ bounded enum/IDs; không dùng user-controlled high-cardinality string hoặc log từng allocation/row.

### 13. Module ownership

- `core`: feature-neutral resource value types, units, requests/envelopes, bounded consumer labels and stable errors. Tokio-backed implementation is local-feature-gated and does not expose Tokio types publicly.
- `config`: validate profiles, quanta, headroom, per-task/plugin limits and storage roots.
- `dag`: validate requested resources/capabilities and declare maximum task working set; không implement allocator.
- `control::admission`: node/task envelope admission, resource class policy, deadlines, placement and pressure/load-shed decisions.
- `runtime`: detect host/cgroup/filesystem capacity, instantiate governors, enable Tokio sync implementation, observe RSS/disk and coordinate startup/shutdown reconciliation.
- `transport`: counted+byte-credit channels; moves accounted buffers/credits without physical double-charge.
- `compute`: native DataFusion memory pool/disk manager/cache integration and metrics.
- `storage`: staging/manifest/Parquet temp leases, durable-capacity transfer and secure cleanup operations.
- `plugins`/`serving`: consume injected envelope/lease handles; không tạo global resource pool.

Không thêm top-level allocator/scheduler crate trong MVP. Nếu feature-neutral `core` boundary không giữ được without engine/runtime types, design phải dừng và mở superseding ADR thay vì re-export Tokio/DataFusion types.

## Consequences

### Positive

- Node/task memory và temp disk có hard admission bounds dễ test.
- Ownership-transfer + RAII khớp Rust/Arrow streaming model và cancellation.
- DataFusion giữ native spill/memory behavior; FurrumX cover các bytes engine không track.
- Channel cap phản ánh payload bytes mà không double-count RSS.
- Crash leftovers trở thành recovery debt trước khi nhận work mới.
- Low-memory node giảm concurrency/spill/load-shed trước OOM.
- Implementation tránh custom allocator, semaphore, spill engine và per-row accounting.

### Negative

- Conservative size/quantization và fixed envelopes có thể giảm utilization.
- Safety headroom làm managed memory nhỏ hơn physical/cgroup limit.
- Correct ownership transfer qua Arrow views và plugin boundaries cần focused tests.
- Same-process Python/native libraries không thể hard-limit hoàn hảo bằng Rust permits.
- External process có thể tiêu thụ disk/RAM ngoài accounting; secondary observation vẫn cần.
- Local feature phải trực tiếp enable minimal Tokio sync/runtime capabilities.

### Compatibility and migration

- Chưa có production resource API nên không cần state/data migration.
- Resource contracts dùng owned domain types; public API chỉ ổn định sau `WP-100/110` review.
- Quanta/profile default có thể tune bằng config/benchmark nếu không thay hard-bound/ownership semantics. Thay từ fixed envelope sang elastic borrowing/custom allocator cần superseding ADR.
- DataFusion/Ballista upgrade phải rerun accounting integration because covered/uncovered allocation boundaries may change.

### Follow-up work

- `WP-100`: resource unit/newtypes, stable errors and curated visibility.
- `WP-110`: governor/lease implementation, profile validation, pressure monitor and leak tests.
- `WP-120`: counted + byte-credit local channel with `AccountedBatch` ownership tests.
- `WP-220/520`: admission/QoS policy on top of hard permits.
- `WP-400/420`: staging/manifest temp leases and restart recovery debt.
- `WP-500`: DataFusion `FairSpillPool`, tracked consumers, disk/cache limits and no-double-charge tests.
- Plugin/serving work packages: Wasmtime/Python/Flight boundary accounting.
- Cargo feature PR: local Tokio `sync`/runtime mapping with ADR-0001 matrix evidence.

## Verification

Decision đã accepted; `WP-110` không hoàn tất cho tới khi có evidence sau.

### Unit/property/concurrency tests

- Checked byte→quantum→Tokio conversions at zero, boundary, `u32::MAX` and overflow.
- Zero-byte lease consumes no unit and cannot authorize positive allocation/file growth.
- Quantization never undercharges; acquire/grow/shrink/drop preserve `reserved + available = limit`.
- Cancellation before/after acquire and pool close do not leak/over-release permits.
- Random split/view/merge/move/drop sequences release physical leases exactly once.
- Arrow slice/projection shares owners; bounded multi-input composition deduplicates lease IDs; materialized output acquires separate lease.
- Count cap and channel credit cap both enforced without adding physical charge twice.
- Pool shrink under live leases enters pressure/wait/fail path without revoke/panic.
- Acquire-order/progress-reserve tests prove bounded pipelines make progress or fail typed error, never hang.
- Loom/model tests for wrapper state if implementation adds atomic state beyond Tokio; no custom unsafe.

### Integration/fault tests

- 2–4 core, 4–8 GiB low profile processes input many times RAM with stable RSS.
- Wide/narrow/skewed batches cannot exceed configured memory/channel bounds.
- Compressed→decoded, Flight IPC and Parquet writer coexistence charges peak correctly.
- DataFusion sort/join/aggregate spills or fails controlled within memory/temp limit; `UnboundedMemoryPool` absent.
- Concurrent DAG + DataFusion queries cannot exceed node envelope or nested vcore/memory limits.
- Temp disk full/external consumption triggers throttle/load-shed before safety reserve.
- Kill during spill/staging/manifest write; restart computes recovery debt and cleanup idempotently.
- Failed delete does not release disk capacity; durable rename transfers rather than erases charge.
- Slow Flight/WS client holds bounded memory/credit and propagates cancellation.
- Wasm/Python/plugin attempts above declared limits fail according to trust/isolation contract.
- Distributed stale controller advertisement is rejected by executor final admission.

### Performance tests

- Microbenchmark permit acquire/try/grow/shrink/drop under 1/2/4/8+ producer contention; report p50/p95/p99, CPU and allocations.
- Benchmark end-to-end parser→Arrow→channel→Parquet with accounting on/off only as diagnostic; production candidate keeps accounting on.
- Report throughput, peak RSS, charged/reserved bytes, headroom delta, backpressure and spill/temp bytes at two dataset sizes.
- Prove accounting overhead is batch/file level and no task/future/allocation per row.
- Compare fixed envelope utilization and fairness across low/standard profiles before considering elastic borrowing.
- Benchmark DataFusion native pool/disk limit rather than custom replacement.

### Required quality gates

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Implementation must run minimal/default plus relevant distributed/Python/Wasm/S3 feature matrix. Hot-path permit/channel/storage changes are `P0-HOT` or `P1-SCALE` and require benchmark manifest under `docs/development/performance-quality-gates.md`.

## References

- [Product requirements](../01-product-requirements.md)
- [System architecture](../02-system-architecture.md)
- [Performance and scalability](../05-performance-scalability.md)
- [ADR-0003: Artifact/checkpoint/manifest ordering](0003-artifact-checkpoint-manifest-ordering.md)
- [Tokio Semaphore](https://docs.rs/tokio/1.48.0/tokio/sync/struct.Semaphore.html)
- [DataFusion MemoryPool](https://docs.rs/datafusion/53.1.0/datafusion/execution/memory_pool/trait.MemoryPool.html)
- [DataFusion FairSpillPool](https://docs.rs/datafusion/53.1.0/datafusion/execution/memory_pool/struct.FairSpillPool.html)
- [DataFusion RuntimeEnvBuilder](https://docs.rs/datafusion/53.1.0/datafusion/execution/runtime_env/struct.RuntimeEnvBuilder.html)
- [Arrow RecordBatch memory size](https://docs.rs/arrow/58.4.0/arrow/array/struct.RecordBatch.html#method.get_array_memory_size)
