# CSV ingestion, schema discovery và schema drift

## 1. Mục tiêu

CSV datasource không chỉ là wrapper quanh một CSV reader. Nó phải là ingestion framework có khả năng:

- Xử lý file từ nhỏ tới một tỷ record.
- Không biết trước encoding, delimiter, header depth hoặc schema.
- Hỗ trợ repeated/multi-row/changing headers.
- Không silent drop malformed data.
- Resume tại safe boundary.
- Land lossless bronze data trước khi normalize kiểu nghiệp vụ.
- Mở rộng sang fixed-width, TSV, log text và các datasource file khác bằng stage abstraction dùng lại được.

## 2. Pipeline ingestion

```text
Acquire immutable raw snapshot
            │
            ▼
Sample and detect encoding/dialect
            │
            ▼
Discover header candidates
            │
            ▼
Plan safe byte/code-unit ranges
            │
            ▼
Decode → record-boundary scan → parse
            │
            ├── inline repeated-header/schema-drift detection
            │
            ├── reject/quarantine stream
            │
            ▼
Lossless Bronze Arrow batches
            │
            ▼
Immutable Parquet parts + checkpoints
            │
            ▼
Typed Silver normalization
```

Discovery không được buộc một full-file pre-scan. Sample trước, phát hiện drift inline trong khi parse.

## 3. File ingestion framework dùng lại được

CSV là plugin MVP đầu tiên, không được hard-code toàn bộ file lifecycle vào `CsvPlugin`. Các stage chung cần abstraction độc lập:

```text
ByteSource
    → SourceSnapshotter
    → FormatDetector
    → PartitionPlanner
    → Decoder
    → RecordFramer
    → SchemaDiscoverer
    → BatchDecoder
    → Validation/Quarantine
    → ArtifactSink
```

Nhờ đó có thể bổ sung TSV, fixed-width, JSON Lines, log formats hoặc spreadsheet adapter mà vẫn dùng chung snapshot, lease, history, checkpoint, reject và commit protocol.

```rust
#[async_trait::async_trait]
pub trait FileFormatPlugin: Send + Sync + 'static {
    fn descriptor(&self) -> &PluginDescriptor;

    async fn detect(
        &self,
        context: PluginContext,
        snapshot: &SourceSnapshot,
        samples: &[ByteSample],
    ) -> EngineResult<FormatDecision>;

    async fn plan(
        &self,
        context: PluginContext,
        snapshot: &SourceSnapshot,
        decision: &FormatDecision,
    ) -> EngineResult<Vec<SourcePartitionPlan>>;

    async fn open(
        &self,
        context: PluginContext,
        partition: SourcePartitionPlan,
        checkpoint: Option<PartitionCheckpoint>,
    ) -> EngineResult<ResumableBatchStream>;
}
```

Format-specific code chỉ chịu trách nhiệm detection/framing/decoding. Các concerns sau thuộc platform core:

- Source identity và raw snapshot.
- Resource admission và byte-accounted buffers.
- Task/partition leases.
- Event history và resume contract.
- Artifact/checkpoint/manifest commit.
- Quarantine retention.
- Metrics và cancellation.

## 4. Raw snapshot và sampling

### 4.1 Sampling windows

Với source seekable:

- Đầu file.
- Một vài window phân bố theo byte size.
- Gần cuối file.
- Window bổ sung quanh anomaly nếu parser phát hiện drift.

Tổng sample target chỉ vài MiB, cấu hình được. Sample metadata lưu byte ranges và score, không nhất thiết lưu raw content vào control DB.

Với stream không seekable:

- Buffer một bounded prefix.
- Resolve encoding/dialect/header trên prefix.
- Replay prefix vào parser.
- Không buffer toàn stream.

Parquet input không nên đi qua stdin/non-seekable buffer vì footer ở cuối; local file/object range reader phải được ưu tiên.

## 5. Encoding detection

### 5.1 Decision order

1. Explicit configuration.
2. BOM detection.
3. Strict UTF-8 validation.
4. UTF-16LE/BE heuristic.
5. `chardetng` guess cho legacy encodings.
6. Candidate decode scoring từ allowlist.
7. Policy fallback hoặc raw-only success.

`chardetng` là browser-oriented detector, không phải authoritative ETL detector. Nó chỉ là một tín hiệu.

### 5.2 Candidate scoring

Mỗi candidate được chấm theo:

- Malformed sequence count.
- Replacement character count.
- Invalid/control character ratio.
- CSV field-count stability.
- Quote balance.
- Header candidate quality.
- Data type consistency ở các dòng sau.
- Optional locale/language hints.

```rust
#[derive(Debug, Clone)]
pub struct EncodingDecision {
    pub decision_id: String,
    pub encoding: String,
    pub method: EncodingDetectionMethod,
    pub sampled_ranges: Vec<ByteRange>,
    pub malformed_sequence_count: u64,
    pub replacement_count: u64,
    pub ambiguity_score: f64,
    pub alternative_candidates: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum EncodingDetectionMethod {
    Explicit,
    Bom,
    ValidUtf8,
    Utf16Heuristic,
    Statistical,
    Fallback,
}
```

### 5.3 Ambiguity policy

- `strict`: fail nếu score gap không đủ lớn.
- `best_effort`: chọn candidate tốt nhất, emit warning và preserve raw.
- `raw_only`: chỉ snapshot raw và chờ user mapping.
- `candidate_fanout`: parse thử nhiều candidate trên sample rồi chọn theo structural score.

Không có policy nào được silent fallback.

## 6. Decoder strategy

`encoding_rs` cung cấp incremental decoder vào caller-provided buffer, phù hợp với bounded streaming.

### 6.1 Fast UTF-8 path

- SIMD UTF-8 validation.
- Nếu valid, parser đọc trực tiếp input bytes.
- Không tạo intermediate UTF-8 `String`.

### 6.2 Shift-JIS và single-byte path

- Reusable decoder output slabs.
- Decode các input chunks thành UTF-8.
- Preserve mapping từ normalized position về raw range ở checkpoint granularity.
- Checkpoint chỉ khi input sequence đã hoàn chỉnh.

### 6.3 UTF-16 path

- Resolve endianness từ BOM/config/heuristic.
- Input range aligned theo hai byte.
- Không cắt surrogate pair.
- Quote/newline scanner hiểu UTF-16 code units hoặc chạy sau bounded transcoding.

### 6.4 Stateful encoding

Nếu decoder state không thể serialize ổn định:

- Tạo durable normalized UTF-8 segment; hoặc
- Replay từ anchor và discard tới checkpoint.

MVP exact byte-boundary resume tập trung UTF-8, UTF-16, Shift-JIS và common single-byte encodings.

## 7. Dialect detection

Candidate delimiters mặc định:

```text
comma
tab
semicolon
pipe
```

Candidate quote/escape/line ending:

```text
quote: double quote, single quote, none
escape: doubled quote, backslash, explicit
line ending: LF, CRLF, CR
```

Score dựa trên:

- Stable field count.
- Balanced quote state.
- Low malformed-record ratio.
- Plausible header/data distinction.
- Sufficient column count.

Multi-character delimiter là slow path riêng; không làm phức tạp hot path một-byte delimiter.

```rust
#[derive(Debug, Clone)]
pub struct CsvDialect {
    pub delimiter: Vec<u8>,
    pub quote: Option<u8>,
    pub escape: Option<u8>,
    pub line_ending: LineEnding,
    pub allow_multiline: bool,
    pub comment_prefix: Option<Vec<u8>>,
}
```

## 8. Header discovery

### 8.1 Các trường hợp phải hỗ trợ

1. Không có header.
2. Header một dòng.
3. Header nhiều dòng.
4. Metadata/title rows trước header.
5. Repeated header giữa file.
6. Header thay đổi giữa file.
7. Duplicate/empty header cells.
8. Các file trong dataset có header/order khác nhau.

### 8.2 Header depth scoring

Thử candidate depth từ `0..max_header_rows`. Score gồm:

- Header cells có tỷ lệ text cao.
- Non-empty/uniqueness ratio.
- Data rows phía sau ổn định về field count.
- Type consistency của data rows.
- Header khác rõ với data rows.
- Composite names có thể normalize.
- Metadata/title rows được tách hợp lý.

Ví dụ:

```text
Report generated at 2026-08-01
Customer,Customer,Revenue,Revenue
ID,Name,JPY,USD
1001,Alice,12000,80
```

Composite names:

```text
Customer + ID     → customer_id
Customer + Name   → customer_name
Revenue + JPY     → revenue_jpy
Revenue + USD     → revenue_usd
```

### 8.3 Stable column identity

```rust
#[derive(Debug, Clone)]
pub struct ColumnIdentity {
    pub column_id: String,
    pub ordinal: usize,
    pub original_labels: Vec<String>,
    pub normalized_name: String,
    pub inferred_type: arrow::datatypes::DataType,
    pub nullable: bool,
}
```

Duplicate names được suffix deterministically:

```text
ID,Name,Name
→ id,name,name__2
```

Không tự gộp cột có label giống nhau. `column_id` chứa header profile fingerprint và ordinal để giữ identity.

### 8.4 No-header fallback

Nếu header không đủ tin cậy:

```text
column_0001
column_0002
column_0003
```

Bronze vẫn được commit và task kết thúc `SucceededWithWarnings`. User có thể cung cấp mapping ở Silver stage mà không ingest lại external source nếu raw/bronze còn giữ.

## 9. Repeated header và source segmentation

Ví dụ:

```text
id,name,amount
1,Alice,100
2,Bob,200
id,name,amount
3,Carol,300
customer_id,customer_name,total
4,David,400
```

Kết quả:

- Repeated identical header: emit event và bỏ khỏi data rows.
- Changed header: đóng segment hiện tại, tạo header profile/schema version/segment mới.

```rust
#[derive(Debug, Clone)]
pub struct SourceSegment {
    pub segment_id: String,
    pub source_snapshot_id: String,
    pub first_raw_offset: u64,
    pub last_raw_offset: u64,
    pub first_record: u64,
    pub last_record: u64,
    pub encoding_decision_id: String,
    pub dialect_id: String,
    pub header_profile_id: String,
    pub schema_version_id: String,
}
```

Inline detector chỉ chạy full scoring khi có trigger:

```text
field count changed
row matched known header signature
type error rate spiked
quote/dialect behavior changed
BOM or explicit report marker appeared
```

Không chạy header classifier nặng trên mọi record.

## 10. Schema inference và evolution

### 10.1 Bronze-first

Bronze giữ dữ liệu lossless, chủ yếu dưới dạng UTF-8/binary-compatible source values cộng provenance:

```text
_source_snapshot_id
_source_segment_id
_source_raw_offset
_source_record_number
_source_encoding
_source_header_profile
_schema_version
_ingested_at
source columns
```

Tùy storage budget, `_source_raw_offset` và `_source_record_number` có thể nằm ở part metadata thay vì từng row. Audit cấp row yêu cầu lưu chúng trong từng row.

### 10.2 Silver typing

- Explicit user schema có ưu tiên cao nhất.
- Inference dựa trên bounded sample và được version hóa.
- Conversion failure không làm mất Bronze value.
- Typed null/reject behavior do policy quyết định.
- Decimal/timestamp/date parsing có explicit formats và locale.

### 10.3 Type widening

```text
Int32 + Int64      → Int64
Int64 + Float64    → Float64
Date + Timestamp   → Timestamp
Missing column     → nullable column
Incompatible types → Utf8 or separate schema version
```

Không tự map semantic aliases như `cust_no` và `customer_id`. Alias registry/user mapping là layer riêng.

### 10.4 Parquet schema versions

Một Parquet part chỉ có một schema. Layout:

```text
datasets/customers/
├── schema=01a.../
│   ├── part-00001.parquet
│   └── part-00002.parquet
├── schema=91b.../
│   └── part-00003.parquet
└── manifests/
    └── 000000000042.arrow
```

Manifest mô tả reconciliation; reader align bằng canonical column identity và nullable missing columns.

## 11. Safe parallel partitioning

### 11.1 Range planning

Tentative ranges theo raw bytes, ví dụ 64–256 MiB. Boundary scanner điều chỉnh đầu/cuối tới complete CSV record.

Một newline không đủ để xác định record boundary vì quoted field có thể chứa newline. Scanner phải duy trì quote/escape state.

### 11.2 Không full pre-index bắt buộc

- Mỗi partition có raw byte identity.
- Parser trả local record count.
- Global logical ordinal có thể được prefix-sum sau.
- Checkpoint identity ưu tiên `(source_snapshot_id, raw_byte_offset)`.

### 11.3 Partition overlap

Nếu boundary discovery dùng look-behind/look-ahead, raw scan ranges có thể overlap nhưng logical ownership không được overlap. Partition manifest ghi rõ owned record start/end.

## 12. Streaming output contract

```rust
#[derive(Debug, Clone)]
pub struct SourcePartitionPlan {
    pub partition_id: u64,
    pub source_snapshot_id: String,
    pub tentative_start_offset: u64,
    pub tentative_end_offset: u64,
    pub owned_start_offset: u64,
    pub owned_end_offset: u64,
    pub expected_encoding: String,
}

#[derive(Debug, Clone)]
pub struct BatchProgress {
    pub partition_id: u64,
    pub first_raw_offset: u64,
    pub next_raw_offset: u64,
    pub first_local_record: u64,
    pub next_local_record: u64,
    pub row_count: u64,
    pub estimated_batch_bytes: u64,
}

pub struct ResumableBatch {
    pub batch: arrow::record_batch::RecordBatch,
    pub progress: BatchProgress,
}
```

Progress watermark chỉ là candidate. Durable checkpoint được tạo sau khi sink commit artifact chứa range tương ứng.

## 13. Invalid row và quarantine

Policies:

- `fail_fast`
- `fail_after_threshold`
- `quarantine`
- `replace_and_warn`
- `raw_only`

Threshold có thể theo count hoặc ratio. Mọi policy phải lưu:

- Raw source coordinate.
- Error code/reason.
- Column/value khi xác định được.
- Raw record bytes hoặc bounded excerpt/reference.
- Encoding/dialect/schema decision IDs.

Không lưu hàng triệu rejects thành SQLite rows; ghi reject Parquet artifact.

## 14. Success semantics

### `Succeeded`

- Mọi record thuộc committed bronze artifacts.
- Không reject/ambiguity vượt zero-warning policy.

### `SucceededWithWarnings`

- Mọi record đã land hoặc quarantine.
- Có fallback encoding/header, schema drift hoặc reject dưới threshold.
- Warning/reject artifacts đầy đủ.

### `Failed`

- Không thể snapshot source.
- Không xác định được record boundary theo policy.
- Corruption/reject vượt threshold.
- Storage/commit failure không phục hồi được.

“ETL thành công” không có nghĩa hệ thống tự hiểu đúng business meaning của mọi column. Nó có nghĩa không mất dữ liệu và tạo được audited, queryable landing state.

## 15. Hot-path constraints

Không dùng trên mỗi row:

- `String` allocation.
- `Vec<String>`.
- `HashMap<String, Value>`.
- Regex.
- Serde.
- Async task/channel message.

Ưu tiên:

- Reusable byte slabs.
- Byte slices/offsets.
- `memchr`/SIMD scanners.
- Column-oriented Arrow builders.
- Direct numeric parsing.
- Batch-level errors/metrics.

## 16. Cấu hình minh họa

```toml
[source]
kind = "csv"
uri = "file:///data/incoming/customers.csv"

[source.snapshot]
mode = "copy_if_mutable"
hash = "blake3"

[source.encoding]
mode = "detect"
candidates = ["utf-8", "utf-16le", "utf-16be", "shift_jis", "windows-1252"]
ambiguity = "best_effort"

[source.csv]
delimiter_candidates = [",", "\t", ";", "|"]
max_header_rows = 5
allow_multiline = true
repeated_header = "segment_or_skip"
schema_drift = "new_segment"

[source.errors]
policy = "quarantine"
max_reject_ratio = 0.001

[sink]
kind = "parquet"
uri = "file:///lakehouse/bronze/customers"
zone = "bronze"
```

## 17. Tài liệu tham khảo kỹ thuật

- [encoding_rs](https://docs.rs/encoding_rs/latest/encoding_rs/)
- [chardetng](https://docs.rs/chardetng/)
- [DataFusion custom TableProvider](https://datafusion.apache.org/blog/2026/03/31/writing-table-providers/)
- [Parquet metadata](https://arrow.apache.org/rust/parquet/file/metadata/index.html)
