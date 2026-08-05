# Yêu cầu sản phẩm và tiêu chí chấp nhận

## 1. Tầm nhìn

FurrumX là một data platform Arrow-native, hiệu năng cao, nhẹ và vận hành bằng một executable Rust duy nhất. Sản phẩm hướng tới việc gom các năng lực thường bị phân tán giữa Airflow, Spark và Kafka vào một hệ thống thống nhất cho ETL/ELT, analytical compute và data serving.

“Single binary” có nghĩa cùng một artifact thực thi được nhiều vai trò. Trong distributed mode, binary vẫn được chạy trên nhiều máy; không có nghĩa toàn cluster chỉ có một process.

## 2. Mục tiêu

- Xử lý file và dataset lớn hơn RAM nhiều lần bằng streaming và spill.
- Chạy tốt trên node nhỏ mà không OOM, đồng thời tận dụng được node lớn.
- Scale-out cho ingest độc lập và distributed query.
- Giảm JVM overhead, GC pause, row-oriented serialization và data conversion không cần thiết.
- Cung cấp orchestration có history, retry, lineage và exact resume tường minh.
- Ingest được CSV không biết trước encoding, header hoặc schema.
- Lưu dữ liệu lossless vào local Parquet trong MVP và object storage/lakehouse về sau.
- Phục vụ BI qua Flight SQL và frontend hiện đại qua Arrow IPC.
- Cho phép mở rộng datasource, sink, Python analytics và sandboxed Wasm logic.

## 3. Phi mục tiêu trong MVP

- Không tái tạo toàn bộ Kafka topic/consumer-group/retention semantics.
- Không hứa exactly-once cho side effect bên ngoài không idempotent.
- Không cung cấp HA controller bằng consensus ngay trong MVP.
- Không gọi manifested Parquet dataset là lakehouse ACID hoàn chỉnh trước khi có snapshot catalog như Iceberg/Delta.
- Không hứa zero-copy qua network, transcoding hoặc Wasm linear memory.
- Không hứa tự động hiểu đúng semantic mapping giữa hai tên cột khác nhau.
- Không cam kết một throughput tuyệt đối nếu không định nghĩa phần cứng, input shape, encoding, codec và storage path.

## 4. Nhóm use case

### 4.1 Batch ingestion

- CSV từ vài KiB tới hàng trăm GiB hoặc một tỷ record.
- Nhiều file cùng logical dataset nhưng khác encoding/header/order/schema.
- Concatenated report có repeated header hoặc header thay đổi giữa file.
- Local filesystem, S3-compatible storage và HTTP source.

### 4.2 ETL/ELT

- Extract → normalize → SQL transform → validate → Parquet commit.
- Chạy các branch độc lập đồng thời.
- Backfill theo source snapshot hoặc time range.
- Resume sau process crash, node loss hoặc storage timeout.

### 4.3 Analytical compute

- SQL projection/filter/join/group-by/window/sort.
- Single-node vectorized execution bằng DataFusion.
- Distributed physical stages bằng Ballista hoặc adapter tương đương.

### 4.4 Custom analytics và ML

- Trusted Python model nhận/trả PyArrow stream.
- Wasm plugin với memory/time/capability limits.
- Rust-native plugin được compile vào binary.

### 4.5 Data serving

- Flight SQL cho JDBC/ODBC/BI client tương thích.
- REST/WebSocket trả Arrow IPC cho browser hoặc Svelte frontend.
- Streaming result, cancellation và resource quota.

## 5. Functional requirements

### 5.1 Orchestration

- `FR-ORCH-001`: Pipeline được mô tả bằng DAG có task ID ổn định và definition version bất biến.
- `FR-ORCH-002`: Scheduler phải phát hiện cycle trước khi submit.
- `FR-ORCH-003`: Task độc lập được chạy đồng thời trong giới hạn resource budget.
- `FR-ORCH-004`: Hỗ trợ retry, exponential backoff, jitter, timeout và cancellation.
- `FR-ORCH-005`: Hỗ trợ cron, manual run, backfill và dataset-triggered run.
- `FR-ORCH-006`: Mỗi retry tạo `TaskAttempt` mới; không ghi đè lịch sử attempt cũ.
- `FR-ORCH-007`: Executor nhận task bằng lease có deadline và heartbeat.
- `FR-ORCH-008`: Stale attempt không được thắng commit sau khi lease/generation không còn hợp lệ.
- `FR-ORCH-009`: Hỗ trợ concurrency group và tenant/resource class.

### 5.2 History, lineage và resume

- `FR-HIST-001`: Mọi task có append-only event history.
- `FR-HIST-002`: Mỗi artifact truy ngược được pipeline version, run, task, attempt, input snapshot, code/config hash.
- `FR-HIST-003`: Checkpoint chỉ được commit sau output artifact tương ứng.
- `FR-HIST-004`: Resume contract phải so khớp source, parser, schema policy, plugin và transform version.
- `FR-HIST-005`: Source mutable phải được snapshot hoặc content-addressed trước khi exact resume.
- `FR-HIST-006`: Hệ thống có reconciler xử lý temporary file, orphan artifact và incomplete commit.
- `FR-HIST-007`: Downstream chỉ đọc artifact thuộc committed manifest generation.
- `FR-HIST-008`: CLI/API hiển thị task timeline, retry reason, checkpoint và lineage.

### 5.3 CSV datasource

- `FR-CSV-001`: Hỗ trợ UTF-8, UTF-16LE/BE, Shift-JIS và common legacy encodings trong allowlist.
- `FR-CSV-002`: Encoding decision có method/evidence/history và có thể override.
- `FR-CSV-003`: Hỗ trợ delimiter, quote, escape, comment, null tokens và multiline quoted fields.
- `FR-CSV-004`: Hỗ trợ no-header, single-row header và multi-row header.
- `FR-CSV-005`: Phát hiện repeated header và schema/header drift trong cùng file.
- `FR-CSV-006`: Duplicate column name được normalize deterministically mà không gộp dữ liệu.
- `FR-CSV-007`: Invalid row không được bỏ âm thầm; phải land hoặc quarantine theo policy.
- `FR-CSV-008`: Tạo source segment và schema version khi dialect/header/schema thay đổi.
- `FR-CSV-009`: Exact resume tại encoding-safe, record-safe boundary.
- `FR-CSV-010`: CSV provider phải hỗ trợ projection/partition pushdown khi khả thi.

### 5.4 Compute và transport

- `FR-COMP-001`: DataFusion là SQL/logical/physical compute engine mặc định.
- `FR-COMP-002`: Query chạy single-node hoặc remote Ballista session qua cùng abstraction.
- `FR-COMP-003`: RecordBatch nội process được chuyển bằng bounded ownership-transfer channel.
- `FR-COMP-004`: Inter-node tabular data dùng Arrow Flight/IPC, không dùng row JSON.
- `FR-COMP-005`: Durable edge dùng immutable IPC/Parquet artifact và manifest.
- `FR-COMP-006`: Partition planner phân phối file range/Parquet row group dựa trên byte weight và vcore allocation.
- `FR-COMP-007`: Controller không proxy tabular data trong distributed ingest.

### 5.5 Storage và sink

- `FR-STOR-001`: MVP ghi local Parquet theo immutable part file.
- `FR-STOR-002`: Sink có `begin`, parallel partition write, `commit` và `abort` lifecycle.
- `FR-STOR-003`: Local commit dùng temporary file, closed footer, flush/fsync và immutable rename.
- `FR-STOR-004`: Object store commit dùng immutable objects và manifest CAS; không giả định atomic rename.
- `FR-STOR-005`: Dataset manifest chứa schema version, object list, statistics, hashes và lineage.
- `FR-STOR-006`: Schema drift được biểu diễn bằng nhiều schema version, không ép lossily vào một Parquet file.
- `FR-STOR-007`: Hỗ trợ raw, bronze, silver và gold zones.

### 5.6 Plugin

- `FR-PLUG-001`: Datasource và sink được đăng ký bằng object-safe Rust traits.
- `FR-PLUG-002`: Rust-native plugin built-in được compile vào binary.
- `FR-PLUG-003`: Runtime third-party plugin ưu tiên Wasm Component Model.
- `FR-PLUG-004`: Python plugin dùng Arrow C Data/C Stream với PyArrow khi cùng process.
- `FR-PLUG-005`: Executor advertise capability để scheduler placement đúng node.
- `FR-PLUG-006`: Plugin manifest chứa API version, artifact hash, capability và resource policy.

### 5.7 Serving

- `FR-SERV-001`: Flight SQL hỗ trợ statement query, metadata, prepared statement và cancellation tối thiểu.
- `FR-SERV-002`: REST query response dùng `application/vnd.apache.arrow.stream`.
- `FR-SERV-003`: WebSocket truyền schema IPC trước record batches.
- `FR-SERV-004`: Query ticket được ký, có tenant, expiration và partition identity.
- `FR-SERV-005`: Slow consumer gây backpressure thay vì tích lũy memory vô hạn.

## 6. Non-functional requirements

### 6.1 Correctness

- `NFR-COR-001`: Không silent data loss.
- `NFR-COR-002`: Không duplicate visible record sau retry/resume.
- `NFR-COR-003`: Checkpoint không được đi trước artifact commit.
- `NFR-COR-004`: Cùng input snapshot và cùng resume contract phải cho deterministic committed result, trừ plugin được khai báo nondeterministic.
- `NFR-COR-005`: Mọi fallback encoding/header/schema đều có history và warning.

### 6.2 Resource safety

- `NFR-RES-001`: RSS không tăng tuyến tính theo tổng số record.
- `NFR-RES-002`: Mọi queue có byte budget.
- `NFR-RES-003`: Khi áp lực RAM/disk tăng, hệ thống phải giảm concurrency, spill, backpressure hoặc load-shed trước OOM.
- `NFR-RES-004`: Node 2–4 core, 4–8 GiB RAM vẫn xử lý được input lớn hơn RAM nhiều lần.
- `NFR-RES-005`: Temporary disk có quota và cleanup/reconciliation.

### 6.3 Performance

- `NFR-PERF-001`: Hot path không tạo `String`/`HashMap` cho từng row.
- `NFR-PERF-002`: Không tạo future/channel message cho từng record.
- `NFR-PERF-003`: Throughput được đo riêng theo raw bytes, decoded bytes, rows và committed Parquet bytes.
- `NFR-PERF-004`: Ingest không-shuffle phải scale gần tuyến tính tới giới hạn shared storage/network.
- `NFR-PERF-005`: Mục tiêu 100+ GB/phút/node chỉ áp dụng cho hardware profile vượt capacity calibration.
- `NFR-PERF-006`: Mục tiêu TB/phút là aggregate cluster target, không phải controller throughput.
- `NFR-PERF-007`: Performance regression trên hot path phải được phát hiện trong benchmark CI.

### 6.4 Availability và recovery

- `NFR-AVL-001`: Process restart phải khôi phục materialized state từ durable history/checkpoints.
- `NFR-AVL-002`: Executor loss chỉ làm lại uncommitted partition range.
- `NFR-AVL-003`: Controller không được là data bottleneck.
- `NFR-AVL-004`: MVP có thể dùng single controller; HA controller là phase sau.

### 6.5 Security

- `NFR-SEC-001`: TLS/mTLS cho Flight và inter-node control.
- `NFR-SEC-002`: Token được validate trên từng RPC/request.
- `NFR-SEC-003`: Python chỉ được chạy với trusted policy.
- `NFR-SEC-004`: Wasm deny-by-default đối với filesystem/network capability.
- `NFR-SEC-005`: Path traversal, arbitrary local URI và secret leakage phải được chặn.
- `NFR-SEC-006`: Query/task có CPU, memory, disk và deadline quota.

## 7. Invariants

- `INV-001`: Không checkpoint nếu artifact chưa committed.
- `INV-002`: Không resume nếu resume contract khác.
- `INV-003`: Không publish manifest tham chiếu object chưa hoàn chỉnh.
- `INV-004`: Không dùng mutable source path làm source identity.
- `INV-005`: Không buffer toàn file hoặc toàn query result trong normal execution.
- `INV-006`: Không đưa controller vào tabular data path.
- `INV-007`: Không dùng `NodeIndex` của petgraph làm durable task identity.
- `INV-008`: Không tuyên bố zero-copy cho transcoding, compute allocation, Wasm hoặc network.
- `INV-009`: Không ghi history/reject theo từng record vào control database.
- `INV-010`: Không trộn Arrow/DataFusion/Ballista major-version universe không tương thích.

## 8. Trạng thái hoàn tất

### `Succeeded`

- Tất cả record đã được xử lý.
- Không ambiguity/reject vượt policy.
- Artifacts và manifest đã commit.

### `SucceededWithWarnings`

- Tất cả record đã land hoặc quarantine.
- Có fallback encoding/header, schema drift hoặc invalid record dưới threshold.
- Warning và reject artifacts đầy đủ.

### `Failed`

- Source snapshot thay đổi.
- Không thể xác lập record boundary an toàn.
- Reject/ambiguity vượt policy.
- Disk/object store/resource bị exhaustion và retry đã hết.
- Manifest commit không thành công.

## 9. Acceptance criteria cấp sản phẩm

- `AC-001`: Ingest CSV một tỷ record với peak RSS nằm trong configured budget.
- `AC-002`: Kill process tại mọi commit stage không tạo duplicate hoặc visible partial output.
- `AC-003`: Thay source/config/plugin version làm checkpoint cũ bị từ chối.
- `AC-004`: File có UTF-8, UTF-16, Shift-JIS, repeated header và changed header đều tạo được audited bronze output.
- `AC-005`: Invalid rows được kiểm đếm và truy xuất từ reject artifact.
- `AC-006`: Single-node và multi-node dùng cùng pipeline definition.
- `AC-007`: Scale-out CSV → Parquet không đi qua controller và đạt efficiency target đã định theo benchmark environment.
- `AC-008`: Slow Flight/WS client không làm RSS tăng không giới hạn.
- `AC-009`: Node thấp xử lý input lớn hơn RAM mà không OOM.
- `AC-010`: CLI/API trả được timeline, checkpoint, artifact lineage và lý do retry của mọi task.
