# ADR-0003: Artifact/checkpoint/manifest ordering và metadata unit-of-work

- Status: Accepted
- Date: 2026-08-05
- Owners: Project owner, storage/recovery maintainers and control-plane maintainers
- Deciders: Project owner, storage/recovery maintainer, control-plane maintainer and performance maintainer
- Related requirements: `FR-HIST-002`, `FR-HIST-003`, `FR-HIST-006`, `FR-HIST-007`, `FR-STOR-001` tới `FR-STOR-005`, `NFR-COR-001` tới `NFR-COR-004`, `NFR-RES-002`, `NFR-RES-005`, `NFR-AVL-001` tới `NFR-AVL-003`, `NFR-SEC-005`, `INV-001`, `INV-003`, `INV-006`, `INV-009`, `AC-002`, `AC-003`, `AC-010`
- Related work packages: `WP-020`, `WP-100`, `WP-200`, `WP-220`, `WP-400`, `WP-410`, `WP-420`, `WP-430`, `WP-620`
- Related decisions: ADR-0001 and ADR-0002
- Supersedes: N/A

## Context

FurrumX thực thi task theo at-least-once nhưng phải cho exactly-once visible dataset effect trong phạm vi manifested dataset. Filesystem/object store, SQLite metadata và task state không cùng một transaction manager. Vì vậy một chuỗi ba lời gọi `commit artifact`, `save checkpoint`, `publish manifest` không tự trở thành atomic và có thể tạo các trạng thái nguy hiểm:

- Checkpoint đi trước output bytes hoàn chỉnh.
- Metadata nói artifact committed nhưng object còn partial hoặc đã bị overwrite.
- Manifest pointer tham chiếu artifact chưa được metadata chấp nhận.
- Manifest đã visible nhưng task/history chưa ghi nhận vì controller crash.
- Stale attempt hoàn tất upload sau khi lease đã được cấp lại.
- CAS conflict làm contributor retry toàn partition và tạo duplicate output không cần thiết.
- Reconciler hoặc GC đoán trạng thái từ filename/age rồi xóa dữ liệu còn được tham chiếu.

Protocol phải giữ controller ngoài tabular data path. Executor/storage adapter ghi Parquet trực tiếp, còn controller chỉ nhận bounded metadata receipt. Metadata write rate phải theo artifact/checkpoint/manifest generation, không theo record hoặc `RecordBatch`.

Repository đã chọn hybrid event ledger và synchronous materialized state trong ADR-0002. ADR này mở rộng typed metadata command của ADR-0002 để artifact facts, checkpoint, lifecycle events và projection transition được commit atomically, đồng thời định nghĩa state machine bên ngoài transaction cho immutable object và manifest publication.

## Scope

ADR này quyết định:

- Khi nào local file hoặc object-store object được coi là physically durable.
- Artifact intent, immutable artifact identity và typed storage receipt.
- Metadata unit-of-work cho artifact, checkpoint, events và watermark.
- Checkpoint coverage, monotonicity, fencing và idempotency.
- Manifest generation planning, MVP serialization và conditional publication.
- Ordering giữa physical object, metadata transaction, external pointer và terminal task state.
- Crash recovery, CAS conflict, reconciliation và garbage-collection roots.
- Module ownership và resource/performance bounds của commit path.

ADR này không quyết định:

- Canonical hash algorithm/ID byte encoding; thuộc `WP-100` nhưng hash phải algorithm-tagged.
- Exact Bronze/Silver/Gold business schema; cần ADR riêng.
- Compaction, merge-on-read, row-level delete/update hoặc lakehouse table semantics.
- Iceberg/Delta catalog adoption; thuộc phase `R6` và cần ADR/migration riêng.
- HA metadata consensus hoặc multi-writer local filesystem.
- Backend-specific durability cao hơn documented object-store success contract.

## Decision drivers

- `INV-001`: checkpoint không được commit trước artifact durable.
- `INV-003`: published manifest không được tham chiếu partial/uncommitted object.
- Stale fencing token không được tạo durable metadata effect hoặc publish pointer.
- Crash tại mọi boundary phải roll forward/cleanup idempotently, không tạo visible gap/overlap/duplicate.
- Filesystem/object store I/O không được chạy bên trong SQLite transaction.
- Normal path không reread toàn artifact chỉ để commit.
- Manifest build không collect toàn object list trong memory.
- Controller metadata/queue rate phải coarse-grained và byte-bounded.
- Tận dụng Arrow/Parquet, `object_store`, Protobuf và SQLite thay vì tự viết format, S3 client hoặc transaction manager tổng quát.
- Local MVP dễ implement/test trước, nhưng contract đủ để adapter S3/R2 dùng conditional writes.

## Options considered

### Option A — Ghi checkpoint trước rồi hoàn tất artifact sau

Ưu điểm:

- Progress xuất hiện sớm.

Nhược điểm:

- Crash có thể resume qua bytes chưa có output.
- Vi phạm trực tiếp `INV-001` và `NFR-COR-003`.

Option này bị từ chối.

### Option B — Xem filesystem/object store và SQLite như một transaction

Thực hiện rename/upload, nhiều repository calls và manifest update trong một application closure rồi gọi đó là transaction.

Ưu điểm:

- API bề ngoài đơn giản.

Nhược điểm:

- Không có atomic commit chung giữa SQLite và storage.
- Giữ SQLite write lock qua filesystem/network I/O làm tăng tail latency và contention.
- Cancellation/crash vẫn tạo trạng thái nửa chừng nhưng không có durable state machine để reconcile.

Option này bị từ chối.

### Option C — Immutable objects + atomic metadata command + manifest pointer CAS

Storage tạo immutable object và receipt trước. Một typed metadata command atomically commit artifact facts, checkpoint, events và watermark. Manifest generation là immutable full snapshot; chỉ conditional pointer update làm generation visible. Reconciler hoàn tất các bước bị gián đoạn.

Ưu điểm:

- Mỗi boundary có authority và recovery action rõ.
- Không giữ DB transaction qua I/O.
- Retry/CAS conflict không cần reprocess data đã committed.
- Dùng trực tiếp Parquet finalization, `object_store` multipart/conditional put và SQLite transaction.
- Reader chỉ cần current pointer + một full-snapshot manifest, không replay chain dài.

Nhược điểm:

- Cần durable intent/publication state machine và fault tests.
- Full-snapshot manifest có write amplification theo số artifact visible, dù build được stream/bound.
- DB confirmation có thể tạm thời đi sau external pointer sau crash.

Option này được chọn.

### Option D — Dùng Iceberg/Delta ngay cho MVP

Ưu điểm:

- Tận dụng table-format transaction, snapshot, schema evolution và catalog ecosystem trưởng thành.

Nhược điểm:

- Tăng dependency/deployment/compatibility surface trước khi local ingest core ổn định.
- Vượt scope manifested Parquet MVP và làm chậm vertical slice đầu tiên.
- Không loại bỏ nhu cầu artifact/checkpoint/fencing semantics riêng của pipeline.

Option này được hoãn tới `R6`. ADR này giữ port boundary để thay manifest publisher bằng catalog transaction sau này mà không đổi checkpoint contract.

## Decision

### 1. Ba commit boundary và authority

Protocol có ba boundary theo thứ tự bắt buộc:

```text
P — Physical durability
    complete immutable artifact bytes outside metadata DB

M — Metadata commit
    atomically record artifact facts + checkpoint + events + projections

V — Visibility publication
    publish immutable manifest generation through current-pointer CAS
```

Ordering bắt buộc:

```text
P < M < V < terminal task transition
```

Ý nghĩa:

- Artifact chỉ `PhysicalComplete` sau boundary `P`; chưa visible cho downstream.
- Artifact chỉ `Committed` sau boundary `M`; checkpoint có thể tham chiếu nó nhưng downstream vẫn không được list/open trực tiếp.
- Artifact chỉ visible khi thuộc generation được current pointer chọn tại boundary `V`.
- `Succeeded`/`SucceededWithWarnings` chỉ được materialize sau khi mọi required output generation đã được xác nhận published trong metadata.

Không dùng wall clock, filename listing hoặc executor acknowledgment làm commit authority.

### 2. Artifact intent và immutable identity

Controller tạo `ArtifactIntent` bằng metadata command trước khi ghi output. Intent chứa tối thiểu:

```text
artifact_id
logical_artifact_key
run_id, task_id, attempt, partition_id
resume_contract_hash
expected source/logical range
artifact_kind and schema_version_id
storage_root_id and relative object path
expected fencing token/generation
intent state and created_at_micros
```

Rules:

- `artifact_id` và object path được cấp một lần cho intent; exact retry dùng lại intent.
- Object path là relative path dưới configured storage root, không phải arbitrary user URI.
- Mỗi attempt/generation mới dùng intent/artifact ID mới. Stale attempt không dùng chung mutable target với current attempt.
- Immutable object được create-only; không overwrite object đã tồn tại.
- `logical_artifact_key` biểu diễn semantic input/range/contract. Nó khác physical `artifact_id`.
- Cùng logical key và cùng canonical artifact facts/content hash chỉ được reuse qua explicit idempotency/cache policy. Cùng logical key nhưng khác content hash là `ARTIFACT_NONDETERMINISTIC_OUTPUT`; không chọn “file mới nhất”.

Intent lifecycle projection:

```text
Preparing → PhysicalComplete → Committed
         ↘ Abandoned
```

`artifact_intents` giữ immutable intent fact; lifecycle trên được materialize vào `artifact_intents_current`. `PhysicalComplete` có thể là transient adapter result; durable database không được coi intent `Committed` trước metadata boundary `M`.

### 3. Physical durability receipt

Storage adapter trả `DurableArtifactReceipt` chỉ sau khi artifact format và storage commit hoàn tất:

```text
artifact_id
logical_artifact_key
storage_root_id
relative_object_path
opaque storage version/ETag when available
content_hash algorithm + bytes
size_bytes
row_count
schema_version_id
source/logical coverage
Parquet footer/schema/statistics summary or bounded references
completion method/capabilities
```

Receipt là owned, bounded metadata. Nó không chứa `RecordBatch`, raw records, full reject rows hoặc unbounded per-column/user metadata.

`ETag`/provider version là opaque concurrency identity, không mặc định là content hash. Strong application content hash được tính incrementally trong write path; normal commit không reread toàn file. Scrub/repair job có thể verify toàn bytes ngoài critical path.

#### Local durable adapter

Linux x86_64/WSL2 local durable profile dùng:

1. Tạo temp file độc quyền trong staging directory trên cùng filesystem với final path.
2. Stream Parquet qua Arrow/Parquet writer với bounded row-group/writer memory.
3. Gọi Parquet `finish`/`close` thành công để ghi footer và lấy metadata.
4. Flush và `sync_all` file; error là commit failure.
5. Close handle rồi atomic no-clobber rename vào unique final path.
6. `sync_all` destination parent directory trước khi phát receipt.

Cross-filesystem rename, overwrite rename và filesystem không cung cấp documented durability primitive không thuộc durable profile. Adapter phải fail capability/startup rõ ràng; không silent fallback sang “best effort”. `object_store::local::LocalFileSystem` có thể dùng cho read/path abstraction nhưng không tự được xem là bằng chứng đã thỏa fsync contract nếu implementation không chứng minh các bước trên.

Implementation không được viết custom unsafe syscall wrapper chỉ để có no-clobber rename/directory sync. Ưu tiên standard library hoặc maintained filesystem crate đã review; dependency PR vẫn phải nêu license, portability, maintenance, binary/compile impact và fault-test evidence.

#### Object-store adapter

Object-store profile dùng Arrow/Parquet async writer integration và `object_store::ObjectStore` thay vì custom S3 client:

1. Stream/multipart upload tới unique intent path với bounded part concurrency/bytes.
2. Finalize Parquet footer và complete upload thành công.
3. Dùng create-only precondition (`PutMode::Create` hoặc backend-equivalent); không upsert immutable key.
4. `HEAD` exact path để bind size và opaque version/ETag; verify provider checksum/application checksum metadata khi capability có.
5. Abort incomplete multipart upload khi cancellation/error; lifecycle rule chỉ là defense-in-depth.

Backend không hỗ trợ create-only immutable object và conditional pointer update không được advertise là publication-capable. Controller không tự implement S3 multipart/retry/signing khi `object_store` đã cung cấp capability đó.

### 4. Typed metadata unit-of-work

Boundary `M` dùng một typed `CommitArtifactCheckpoint` command theo ADR-0002. Không expose SQL transaction/connection hoặc generic `Vec<Mutation>` cho application caller.

Command chứa tối thiểu:

```text
command_id and canonical command_hash
run/task/attempt/partition identities
expected attempt/projection revisions
expected fencing token/generation
one or more validated DurableArtifactReceipt values
checkpoint proposal and previous checkpoint revision
bounded lifecycle events
optional terminal-publication prerequisites, never terminal success itself
```

Control commit coordinator thực hiện validation/storage inspection ngoài transaction, rồi gửi một command cho serialized metadata writer. Trong một SQLite `BEGIN IMMEDIATE` transaction, writer:

1. Resolve exact command retry/conflict.
2. Verify expected attempt revision, lease and fencing token.
3. Verify every receipt matches a live intent, storage root/path, logical key, schema, range and contract.
4. Insert immutable artifact facts and materialize matching intent state as committed.
5. Validate checkpoint monotonicity/coverage and insert checkpoint + artifact links.
6. Append `ArtifactCommitted`/`CheckpointCommitted` and related bounded events.
7. Advance partition/attempt watermark and materialized lifecycle state.
8. Persist commit receipt and commit revision.
9. Commit, then acknowledge caller.

Nếu bất kỳ receipt/checkpoint/event/constraint nào invalid, toàn command rollback. Không filesystem/network I/O, Parquet parse, full hash scan hoặc long CPU work trong transaction.

Implementation ban đầu dùng một explicit command variant. Không tạo framework plugin participant/two-phase transaction tổng quát; command variant mới chỉ được thêm khi có invariant và owner rõ.

### 5. Checkpoint validity và coverage

Checkpoint là immutable fact và monotonic trong một partition + resume contract. Nó chứa/links tối thiểu:

```text
checkpoint_id and checkpoint_sequence
run/task/attempt/partition
resume_contract_hash
previous_checkpoint_id/revision optional
next safe raw/logical coordinates
committed data/reject artifact IDs
coverage summary/hash
fencing generation
metadata commit revision
```

Một checkpoint chỉ hợp lệ khi:

- Tất cả artifact bắt buộc đã qua `P` và được insert `Committed` trong cùng command `M` hoặc đã committed trước đó.
- Coverage từ previous checkpoint tới next coordinate không có gap/overlap; mọi logical record đã land hoặc quarantine theo declared policy.
- Decoder/record boundary là safe theo resume contract.
- Checkpoint sequence/revision tăng, không giảm offset hoặc đổi contract.
- Attempt/fencing generation còn current tại commit.
- Data artifact và required reject/warning artifact cùng được bind trước khi watermark advance.

DB dùng foreign key, `UNIQUE`, `CHECK` và compare-and-swap làm defense-in-depth. Validation cấp domain nằm ở `checkpoint`; SQLite adapter không tự phát minh business transition bằng trigger.

### 6. Manifest plan và generation state machine

Manifest publication là state machine riêng, không nằm trong `CommitArtifactCheckpoint` transaction:

```text
Preparing → Eligible → Published
         ↘ Abandoned/Superseded
```

`PrepareManifestGeneration` metadata command:

- Chọn `dataset_id`, new `generation: u64` và expected parent generation/pointer revision.
- Lưu bounded change set gồm artifact adds/removes; artifact add phải đã `Committed`.
- Validate dataset/schema/partition policy và authorization.
- Tạo canonical `publication_plan_hash` và deterministic sort contract.
- Insert immutable generation plan/change facts và materialize current generation state as `Preparing`; không làm generation visible.

MVP manifest generation là **full snapshot Arrow IPC file**, không phải delta chain. Lý do:

- Arrow IPC đã nằm trong BOM/local feature và có schema/versioned metadata.
- Entry table là columnar metadata và được ghi/đọc theo bounded `RecordBatch`.
- Reader mở một generation trực tiếp, không replay chain dài.
- Không thêm JSON/Serde per-entry path hoặc tự thiết kế binary container.

Manifest writer stream current artifact projection + planned changes theo deterministic order vào bounded Arrow IPC batches và temporary-disk quota; không collect toàn object list. Generation object được create-only bằng cùng immutable-object protocol. Full-snapshot write amplification phải được đo; nếu object count vượt supported threshold, compaction/catalog evolution phải qua benchmark và ADR, không đổi âm thầm sang unbounded delta chain.

Schema metadata/rows chứa tối thiểu:

```text
format_version
dataset_id, generation, parent_generation optional
publication_plan_hash and eligible metadata commit revision
artifact_id, storage_root_id, relative_object_path, opaque object version optional
content hash algorithm/value, size_bytes, row_count
schema_version_id and bounded partition/source coverage
bounded pruning statistics or immutable statistics reference
lineage reference
```

Không persist resolved credential, arbitrary local URI, raw row hoặc secret. Persistent counts/offsets dùng `u64`; Arrow physical integer types phải preserve range.

`SealManifestGeneration` metadata command chỉ materialize `Preparing → Eligible` khi immutable generation receipt có exact generation ID, plan hash, byte hash, object path, entry count/summary và expected parent. Seal không publish pointer.

Logical metadata tables của ADR này:

```text
Immutable facts:
  artifact_intents
  artifacts
  checkpoints
  checkpoint_artifacts
  manifest_generations
  manifest_generation_changes

Materialized current projections:
  artifact_intents_current
  manifest_generations_current
  dataset_publications_current
```

Current projection rows chỉ được update bởi deterministic materializer/typed metadata command theo ADR-0002; caller không CRUD trực tiếp.

### 7. Current pointer và publication CAS

Mỗi dataset có một bounded `current` pointer Protobuf object:

```text
format_version
dataset_id
generation
manifest storage root/path
manifest content hash
publication_plan_hash
eligible metadata commit revision
```

Pointer không chứa object list. Writer publish một `Eligible` generation bằng:

- First publication: create-only pointer.
- Existing publication: conditional update dùng exact previous opaque version/ETag (`PutMode::Update(UpdateVersion)` hoặc backend-equivalent).
- Local MVP: one active controller + exclusive dataset publication lock; validate expected parent, write/sync temp pointer, atomic replace, rồi sync parent directory.

Unconditional pointer overwrite bị cấm. Backend thiếu atomic/conditional pointer capability không được advertise support cho manifested write.

Executor/plugin không được gọi pointer publisher. Task-attempt fencing được tiêu thụ tại boundary `M`; sau khi artifact/checkpoint đã commit hợp lệ, controller-owned publisher được phép roll forward generation dù executor lease sau đó hết hạn. Trước CAS/confirm, publisher vẫn phải validate generation eligibility, expected parent/dataset revision và active-controller authority. HA leader fencing ngoài single-controller MVP phải được bổ sung bằng ADR backend tương lai.

Sau CAS thành công, control gửi `ConfirmManifestPublished` metadata command. Command verify exact generation/hash/plan and expected dataset revision, rồi trong một transaction:

- Materialize generation state as `Published`.
- Update `dataset_publications_current` và materialized artifact visibility.
- Append `ManifestPublished` event.
- Persist opaque publication receipt/version.
- Transition task terminal nếu và chỉ nếu tất cả required outputs/checkpoints/warnings đã published.

External current pointer là authority cho dataset visibility; confirmed metadata là authority cho scheduler/history. Metadata không bao giờ được dẫn trước pointer. Nếu crash sau pointer CAS nhưng trước confirm, reader có thể thấy valid new snapshot, còn reconciler phải confirm metadata trước khi scheduling tiếp tục.

### 8. CAS conflict và retry

Khi pointer CAS fail:

1. Đọc exact current pointer; không dựa vào list order/timestamp.
2. Nếu pointer đã trỏ đúng generation/hash/plan, coi là exact retry và chạy confirm idempotently.
3. Nếu pointer trỏ generation khác, mark candidate `Superseded` và rebase change set lên latest confirmed parent.
4. Tạo generation/plan mới, rebuild full manifest và retry với bounded backoff/deadline.

Không rerun parser/transform hoặc rewrite committed data artifact chỉ vì manifest CAS thua. Add/remove conflict không merge âm thầm: policy phải trả typed conflict nếu cùng logical dataset partition/range bị thay đổi không tương thích.

Mọi metadata step dùng stable `command_id`/hash semantics của ADR-0002. Storage create/CAS retry dùng artifact/generation identity và exact content/plan hash; retry payload khác cùng identity là conflict.

### 9. Crash recovery và reconciliation

Reconciler đọc durable intents/facts/pointers và dùng cùng typed commands/ports như normal path. Không update ad-hoc SQL hoặc đoán từ mtime.

| Crash point | Durable observation | Recovery action |
|---|---|---|
| Trước artifact intent | Không có intent | Retry work allocation |
| Sau intent, trước temp/upload | `Preparing`, không object | Retry hoặc abandon sau lease/grace checks |
| Trong temp/multipart | Partial staging/upload | Abort/delete sau lease expiry; không promote bằng filename |
| Sau Parquet finish, trước physical commit | Complete staging candidate | Validate exact intent/fence rồi finish, hoặc cleanup |
| Sau immutable object, trước metadata `M` | Orphan physical object + live/stale intent | Commit bằng same command nếu still authorized; nếu không, mark GC candidate |
| Sau `M`, trước manifest plan | Committed, not visible artifact/checkpoint | Resume publication; không reprocess range |
| Sau manifest plan/object, trước seal | Preparing generation/orphan candidate | Verify exact plan/hash rồi seal hoặc abandon |
| Sau seal, trước pointer CAS | Eligible generation | Retry CAS |
| Sau pointer CAS, trước metadata confirm | Pointer ahead of metadata | Verify pointer/hash then idempotent confirm |
| CAS conflict | Superseded candidate | Rebase metadata change set; do not rewrite artifacts |
| Sau confirm | Published generation | No-op exact retry |

Startup writable readiness phải reconcile pointer/metadata divergence trước khi lease/commit mới. Corrupt/unknown manifest or pointer version, missing committed artifact, hash mismatch hoặc ambiguous ownership làm dataset/controller fail closed; không skip artifact rồi publish phần còn lại.

### 10. Garbage collection và retention

- GC roots gồm current/retained published manifest generations, committed checkpoints, active intents, legal holds và configured raw/source retention.
- Unpublished candidate manifest và orphan artifact chỉ được sweep sau durable GC mark, lease expiry, grace period và recheck roots/version ngay trước delete.
- Delete dùng exact storage root/path và opaque version/precondition khi backend hỗ trợ; không recursive delete user-controlled prefix.
- Incomplete multipart upload cleanup dùng adapter abort + provider lifecycle policy defense-in-depth.
- Reconciler/GC command phải có dry-run/audit summary và bounded page/batch processing.
- Không delete solely theo filename age; không xóa object còn được checkpoint hoặc retained generation tham chiếu.

### 11. Module ownership

- `core`: durable IDs, generations/revisions/fencing newtypes, algorithm-tagged hashes, storage-root/object-ref value types và stable errors.
- `storage`: Parquet finalization, local durable file adapter, `object_store` adapter, artifact/manifest receipts, Arrow IPC manifest codec, current-pointer publisher và physical cleanup operations.
- `checkpoint`: resume contract, checkpoint proposal, safe-boundary/coverage/monotonic validation.
- `history`: ADR-0002 metadata writer, relational fact rows, typed `CommitArtifactCheckpoint`/manifest-state command variants and lifecycle materialization. Command DTO chỉ dùng owned/core scalar types; không expose SQLite rows/connection.
- `control::commit`: orchestration, authorization, receipt inspection, mapping validated domain values into metadata commands, publication retry/rebase and terminal transition request.
- `compute`: manifest-backed reader/provider; không list storage prefix để tạo dataset snapshot.
- `runtime`: wire adapters/workers/queues and startup reconciliation order; không chứa business commit rules.
- `plugins`/executors: stream data và trả receipt; không advance checkpoint, publish pointer hoặc mark task terminal.

Không thêm top-level `metadata`/`catalog` module trong MVP chỉ để bọc SQLite. Nếu history fact surface tăng tới mức ownership không còn rõ, tách crate/module phải qua ADR với migration plan.

### 12. Resource, HPC và observability contract

- Artifact bytes đi executor → storage trực tiếp; controller chỉ nhận bounded receipts.
- Parquet/manifest writers có byte permits, writer concurrency và temporary-disk quota.
- Multipart part size/count/concurrency bounded; cancellation aborts upload.
- Manifest entry iteration/page/batch bounded; không `Vec` toàn dataset.
- Metadata command có maximum artifact/event/count/bytes; writer queue reservation tính serialized command size.
- Event/checkpoint rate là `O(artifacts + partitions)`, không `O(records + batches)`.
- Manifest publish retry/backoff/CAS attempts bounded; conflict storms load-shed rõ ràng.
- Metrics tối thiểu: artifact finish/fsync/upload latency, bytes/rows, metadata commit p50/p95/p99, manifest build bytes/entries/time/RSS, pointer CAS conflict, reconciliation lag/orphan bytes, GC candidates/deletes và controller CPU/queue bytes.
- Không giảm fsync/checksum/CAS trong benchmark default. Unsafe/fast durability mode không được đại diện production claim.

## Consequences

### Positive

- Checkpoint không thể dẫn trước durable artifact.
- Manifest pointer không thể hợp lệ nếu generation chưa được seal từ committed facts.
- Crash/CAS conflict có deterministic roll-forward mà không reprocess data đã committed.
- Reader có snapshot đơn giản: pointer nhỏ + một immutable full manifest.
- Control plane không nhận tabular bytes và metadata rate giữ coarse-grained.
- Implementation reuse Arrow IPC, Arrow/Parquet writer, `object_store` conditional/multipart operations, Protobuf và SQLite transaction.
- Storage/catalog backend tương lai có port boundary rõ để thay thế.

### Negative

- Không có atomic transaction chung giữa pointer và SQLite; reconciliation là bắt buộc.
- Full-snapshot manifest tạo metadata write amplification theo artifact count.
- Durable local path cần filesystem-specific sync/rename tests và không mặc định support mọi mount/network filesystem.
- Manifest/checkpoint schema và state machine làm implementation/fault-test surface lớn hơn CRUD.
- Arrow IPC manifest và Protobuf pointer trở thành durable formats cần compatibility/golden tests.

### Compatibility and migration

- Chưa có production artifact database/manifest nên decision ban đầu không cần data migration.
- Sau migration/format version đầu tiên, field/schema change phải có backward reader, explicit migration hoặc superseding ADR.
- Older binary gặp newer unsupported pointer/manifest format phải fail writable/read clearly; không reinterpret hoặc list object prefix để fallback.
- Iceberg/Delta adoption phải import/preserve artifact IDs, content hashes, lineage và checkpoint references; không gọi manifested MVP là ACID lakehouse trước đó.

### Follow-up work

- `WP-100`: IDs, `ContentHash`, object/version refs, generation/revision and stable error codes.
- `WP-200`: metadata migrations/typed command variants/idempotency from ADR-0002/0003.
- `WP-400`: local durable Parquet writer and storage capability tests.
- `WP-410`: artifact intent, checkpoint coverage and commit coordinator.
- `WP-420`: Arrow IPC manifest codec, Protobuf pointer, CAS publisher, reconciler and GC.
- ADR-0004: byte-accounted memory/temporary-disk permits including manifest and multipart writers.
- Bronze/Silver/Gold ADR: schema/publish policy without changing this ordering.
- Catalog ADR in `R6`: evaluate Iceberg/Delta with measured migration/operational cost.

## Verification

Decision đã accepted; `WP-410`/`WP-420` không hoàn tất cho tới khi có evidence sau.

### Contract/golden/property tests

- Canonical logical artifact key, publication plan, pointer Protobuf and Arrow IPC manifest golden files.
- Compatible/unknown pointer and manifest format versions.
- Checked `u64` counters/offsets/generations; no truncate/wrap in SQLite/Arrow adapters.
- Deterministic manifest bytes/plan hash for same sorted facts and writer version contract.
- Checkpoint coverage property tests: arbitrary partitions/ranges never allow gap/overlap/regression.
- Same logical key/same facts exact retry; different content returns nondeterminism conflict.

### Fault/concurrency tests

- Kill/fault at every row in the crash matrix before/after fsync, rename, metadata commit, manifest seal, pointer CAS and confirmation.
- Cancellation before/after every boundary.
- Stale executor/fencing token không thể commit artifact/checkpoint hoặc trực tiếp publish/confirm generation; controller vẫn roll forward facts đã commit hợp lệ sau executor lease expiry.
- Two concurrent attempts and two manifest publishers: one CAS wins; loser rebases or fails typed conflict without duplicate visible records.
- Pointer-ahead-of-metadata startup reconciliation.
- Missing/corrupt object, footer, manifest, pointer, content hash and opaque version fail closed.
- Exact retry after lost acknowledgment returns same receipt/generation.
- Reconciler and GC repeated runs are idempotent; retained roots are never deleted.
- Local filesystem tests cover same-filesystem no-clobber rename, file sync, parent sync and unsupported mount behavior.
- Object-store tests use reusable `object_store` integration/fault adapter plus S3-compatible conditional/multipart failure injection.

### Resource/performance tests

- Artifact/manifest write path has bounded memory at two artifact-count/data sizes.
- Manifest build streams entries and reports peak RSS/allocation; no full object-list collection.
- Benchmark artifact finish/fsync/upload, metadata commit and manifest publish separately with p50/p95/p99.
- Report controller event/command rate, CAS conflicts, storage request amplification, temp bytes and output size.
- Compare full-snapshot manifest cost at representative artifact counts; define supported threshold before release.
- Durability benchmark keeps file sync, parent sync, strong hash and conditional publication enabled.

### Required quality gates

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

S3 implementation phải chạy relevant feature matrix và conditional/multipart integration suite. Hot-path/storage optimization phải theo `docs/development/performance-quality-gates.md` với benchmark manifest.

## References

- [Product requirements](../01-product-requirements.md)
- [System architecture](../02-system-architecture.md)
- [History, lineage và exact resume](../03-history-resume-lineage.md)
- [ADR-0002: Event store và materialized state](0002-event-store-materialized-state.md)
- [object_store conditional put and multipart API](https://docs.rs/object_store/0.13.1/object_store/)
- [Apache Arrow Rust Parquet async writer](https://arrow.apache.org/rust/parquet/arrow/async_writer/struct.AsyncArrowWriter.html)
- [Amazon S3 conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)
- [SQLite transactions](https://www.sqlite.org/lang_transaction.html)
- [SQLite WAL](https://www.sqlite.org/wal.html)
