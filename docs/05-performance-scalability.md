# Hiệu năng, khả năng chịu tải và scale-out

## 1. Performance philosophy

“Cực kỳ trâu” được định nghĩa bằng bốn thuộc tính:

1. Dataset lớn tùy ý không làm memory tăng theo tổng record count.
2. Hệ thống tiến gần bottleneck vật lý thấp nhất thay vì tự tạo bottleneck phần mềm.
3. Khi thiếu tài nguyên, throughput giảm có kiểm soát thay vì OOM/crash/corrupt output.
4. Scale-out không đưa bytes qua controller và giữ được correctness/recovery semantics.

Absolute throughput chỉ có ý nghĩa khi đi cùng:

- Raw/compressed input size.
- Average row width và column count.
- Encoding/dialect.
- Transform complexity.
- Parquet codec.
- CPU/memory/disk/network.
- Source/sink storage.
- Concurrency và durability policy.

## 2. Throughput metrics

Báo cáo riêng:

```text
raw_ingress_bytes_per_second
decoded_bytes_per_second
parsed_records_per_second
arrow_output_bytes_per_second
parquet_committed_bytes_per_second
flight_sent_bytes_per_second
flight_received_bytes_per_second
```

Quy đổi decimal:

```text
100 GB/min ≈ 1.67 GB/s
300 GB/min ≈ 5.00 GB/s
1 TB/min   ≈ 16.67 GB/s
```

Không so sánh raw compressed bytes với decoded logical bytes mà không ghi rõ.

## 3. Capacity model

Node throughput:

```text
R_node = min(
    source_read_bandwidth,
    decode_throughput,
    record_scan_throughput,
    csv_parse_throughput,
    transform_throughput,
    parquet_encode_throughput,
    sink_write_bandwidth,
    network_bandwidth
)
```

Cluster throughput:

```text
R_cluster ≈ efficiency × min(
    sum(R_node),
    shared_storage_bandwidth,
    network_bisection_bandwidth,
    shuffle_bandwidth
)
```

Nếu source và sink dùng chung disk, cùng một byte input có thể tạo cả read và output write traffic. Target 100 GB/phút có thể đòi tổng device traffic vượt nhiều GB/s.

Ví dụ CPU budget: 4 core × 3 GHz tại 1,67 GB/s chỉ có khoảng 7,2 cycle/byte cho toàn bộ decode, parse, conversion, checksum và Parquet encode. Vì vậy target hàng trăm GB/phút không thể áp cho mọi low-end node.

## 4. Hard performance invariants

- Không `collect()` toàn input/query trong normal path.
- Không allocation per-row trên hot path.
- Không task/future/message per-row.
- Mọi channel có byte budget.
- Mọi parser/writer có concurrency cap.
- Controller nhận metadata aggregate, không nhận batch.
- Checkpoint/history rate là `O(partitions + artifacts)`, không phải `O(records)`.
- Parquet part được đóng hoàn chỉnh; không append-resume partial file.
- Cache có quota và evictable.
- DataFusion memory pool không phải memory budget duy nhất; transient batches cũng được accounting bên ngoài.

## 5. Staged execution pipeline

```text
Source range readers
        │ byte slabs
        ▼
Encoding decoder lanes
        │ normalized byte slabs
        ▼
Boundary scanner lanes
        │ owned record ranges
        ▼
CSV parser lanes
        │ Arrow builders
        ▼
Bounded RecordBatch transport
        │
        ▼
Transform/DataFusion
        │
        ▼
Parquet writer lanes
        │ immutable part files
        ▼
Artifact/checkpoint commit
```

Mỗi stage xuất metrics:

- Queue bytes/current capacity.
- Active workers.
- Input/output rate.
- Processing CPU time.
- Backpressure time.
- Allocation/peak memory.
- Errors/retries.

Stage có queue đầy là bottleneck hiện tại; upstream phải tự dừng đọc.

## 6. Buffer and memory accounting

```rust
#[derive(Debug, Clone)]
pub struct ExecutorResourceBudget {
    pub total_memory_bytes: u64,
    pub datafusion_memory_bytes: u64,
    pub input_buffer_bytes: u64,
    pub batch_queue_bytes: u64,
    pub parquet_writer_bytes: u64,
    pub plugin_memory_bytes: u64,
    pub emergency_headroom_bytes: u64,
    pub temporary_disk_bytes: u64,
    pub compute_vcores: u16,
}
```

Phân bổ baseline:

| Consumer | Tỷ lệ gợi ý |
|---|---:|
| DataFusion/operators | 40–50% |
| Input/decode/parser | 10–15% |
| Arrow edge queues | 10–15% |
| Parquet writers | 15–20% |
| Control/plugin/other | 5–10% |
| Emergency headroom | 10–20% |

Tỷ lệ phải tự điều chỉnh theo workload. DataFusion chỉ accounting các memory consumer lớn đã tích hợp; batches/network/plugin vẫn cần external permits.

### 6.1 Byte-accounted batch

```rust
#[derive(Clone)]
pub struct MemoryPermit {
    pub reserved_bytes: usize,
    pub pool: std::sync::Arc<MemoryPermitPool>,
}

pub struct AccountedBatch {
    pub batch: arrow::record_batch::RecordBatch,
    pub permit: MemoryPermit,
}
```

Permit đi cùng batch và được trả khi downstream drop. Queue capacity theo cả message count và in-flight bytes.

### 6.2 Pressure response

Theo thứ tự:

1. Ngừng mở partition mới.
2. Giảm parser/writer concurrency.
3. Giảm target batch size.
4. Flush writer sớm hơn.
5. Spill operator hỗ trợ spill.
6. Evict cache.
7. Pause source bằng backpressure.
8. Load-shed query/run mới.
9. Fail task rõ ràng trước khi OS OOM-kill.

## 7. Adaptive batch sizing

Batch target theo byte, không chỉ row:

| Profile | Target RecordBatch |
|---|---:|
| Low memory | 4–16 MiB |
| Standard | 16–64 MiB |
| Large | 32–128 MiB |

Flush khi một điều kiện đạt:

```text
estimated_batch_bytes >= target_batch_bytes
OR rows >= max_batch_rows
OR memory pressure requested flush
OR source segment/schema changed
```

Row limit gợi ý: 32K–256K, nhưng wide rows có thể flush sớm hơn nhiều.

## 8. CPU execution

- Tokio: I/O, timers, scheduler, network, lifecycle.
- DataFusion: vectorized query operators.
- Rayon/fixed CPU pool: custom CPU logic không thuộc DataFusion.
- `spawn_blocking`: bounded blocking library calls, không dùng làm unlimited compute scheduler.
- BLAS/Python/OpenMP thread count phải bị giới hạn.

Admission controller cấp vcore envelope. Không tạo một CPU pool trong mỗi plugin/query.

SIMD strategy:

- Dùng Arrow/DataFusion kernels trước.
- Fast UTF-8 validation và byte scanning có runtime SIMD.
- Benchmark portable baseline và CPU-specific build.
- `target-cpu=native` chỉ dùng cluster phần cứng đồng nhất.

## 9. CSV fast path

- UTF-8 valid, one-byte delimiter và seekable source.
- mmap/range read immutable local file.
- Quote-aware block scanner.
- Byte slices vào column builders, tránh intermediate rows.
- Direct numeric/date parsing.
- Parallel safe ranges.
- Content hash cập nhật trong same pass.

Legacy encoding path tốn transcoding allocation; throughput target phải tách riêng theo encoding.

## 10. Parquet throughput profile

### 10.1 File and row-group sizing

| Profile | Part file | Row group uncompressed |
|---|---:|---:|
| Low memory | 128–512 MiB | 32–128 MiB |
| Standard | 512 MiB–2 GiB | 64–256 MiB |
| Large/distributed | 1–4 GiB | 128–512 MiB |

Actual size phải benchmark theo row width, query pruning và recovery granularity.

### 10.2 Codec strategy

- Fast bronze ingest: Snappy/LZ4 hoặc no compression theo storage budget.
- Balanced: low-level Zstd.
- Archive: higher Zstd trong asynchronous compaction.

Không dùng compression nặng trên critical ingest path rồi kỳ vọng maximum raw throughput.

### 10.3 Dictionary

Adaptive dictionary theo:

- Dictionary bytes.
- Distinct count.
- Hit ratio.
- Per-column memory budget.

Fallback plain encoding với high-cardinality IDs.

### 10.4 Query optimization

- Partition pruning.
- Projection pushdown.
- Row-group statistics pruning.
- Page index/bloom filter khi phù hợp.
- Partial aggregate trước shuffle.
- Sort/order metadata để tránh sort thừa.

DataFusion đã tối ưu pushdown/pruning; custom provider phải báo capability chính xác thay vì tự filter chậm hơn vectorized `FilterExec`.

## 11. Out-of-core

- `FairSpillPool` hoặc bounded/tracked DataFusion pool.
- Dedicated local NVMe spill directory khi có.
- Temporary disk quota theo tenant/task.
- Spill file validation/cleanup.
- mmap cho immutable local source, không cho mutable/truncatable file.
- Remote object storage dùng range/vectored reads, không mmap.

mmap sử dụng OS page cache nhưng compressed Parquet pages vẫn phải decode. Nó giảm heap copy/read overhead trong một số path, không làm query zero-copy.

## 12. Distributed ingest

CSV → Parquet không cần shuffle:

```text
Controller creates source-range manifest
    ├── Executor A reads ranges and writes parts directly
    ├── Executor B reads ranges and writes parts directly
    └── Executor C reads ranges and writes parts directly
Controller commits merged metadata manifest
```

Partition lease target: 128 MiB–2 GiB raw range tùy node/network/checkpoint cost. Nhiều file nhỏ được bundle để giảm scheduler overhead.

### 12.1 Work stealing

- Executor pull partition khi còn permits.
- Node rảnh lấy unleased work.
- Lease hết hạn cho phép retry.
- Speculative execution chỉ cho deterministic/idempotent partition.
- First valid fencing-token/CAS commit wins.

### 12.2 Data locality

Placement order:

1. Node có local source/cache.
2. Node cùng storage zone.
3. Node có network/disk capacity cao.
4. Node phù hợp encoding/plugin capability.

### 12.3 Shuffle workloads

Với join/group-by/sort:

- Partition theo key/range.
- Filter/projection trước network.
- Partial aggregate trước shuffle.
- Detect skew/hot partition.
- Split skewed partitions hoặc salt key theo plan.
- Flight IPC compression chỉ bật khi network là bottleneck và CPU còn dư.

## 13. Flight backpressure

```rust
#[derive(Debug, Clone)]
pub struct StreamBudget {
    pub max_in_flight_bytes: u64,
    pub max_batch_bytes: u64,
    pub idle_timeout_ms: u64,
    pub total_timeout_ms: u64,
}
```

Application credit flow:

```text
receiver memory permit
    → grants credits
    → sender sends bounded batches
    → downstream queue fills
    → credits stop
    → sender stops upstream consumption
```

Không chỉ dựa vào HTTP/2 window; decoded application buffers vẫn cần accounting.

## 14. Metadata/history at scale

History complexity:

```text
O(tasks + partitions + artifacts + checkpoints)
```

Checkpoint interval gợi ý:

```text
256 MiB–2 GiB committed range
OR 10–60 seconds
```

Tại 1 TB/phút, artifact/checkpoint size 1 GiB chỉ tạo khoảng 17 commit metadata operations/s toàn cluster, trước batching. Không tạo event theo batch/record.

Rejects được ghi Parquet; event chỉ chứa count/reference/histogram.

## 15. QoS và load shedding

Resource classes:

| Class | Mục tiêu |
|---|---|
| Interactive | Low-latency BI/API |
| Batch | Throughput ETL |
| Maintenance | Compaction/stats/vacuum |

Weighted fair scheduling và reserved capacity ngăn ETL một tỷ row làm Flight SQL mất phản hồi.

Khi quá tải:

- Queue có giới hạn.
- HTTP 429 hoặc gRPC `RESOURCE_EXHAUSTED`.
- `Retry-After`/backoff hint.
- Tenant concurrency quota.
- Query deadline/cancellation.
- Không degrade correctness để nhận thêm work.

## 16. Node profiles

### 16.1 Low

```text
2–4 cores
4–8 GiB RAM
1–2 parser workers
1 Parquet writer
4–16 MiB batches
128–512 MiB parts
minimal cache
aggressive backpressure
```

Cam kết: xử lý input lớn hơn RAM, RSS bounded, resume được. Không cam kết 100 GB/phút.

### 16.2 Standard throughput

```text
8–32 cores
32–128 GiB RAM
parallel parser/writer lanes
16–64 MiB batches
512 MiB–2 GiB parts
NVMe spill
```

Đây là tier có thể hướng tới 100+ GB/phút nếu startup calibration xác nhận source/CPU/sink bandwidth.

### 16.3 Distributed throughput

- Nhiều executor.
- Shared object storage hoặc distributed local sources.
- Direct executor-to-storage path.
- Hierarchical/batched metadata.
- Mục tiêu TB/phút khi aggregate bandwidth đáp ứng.

## 17. Startup calibration

Executor chạy bounded calibration hoặc đọc cached result:

- Sequential read/write.
- Temp disk spill.
- Memory copy.
- UTF-8 validation.
- Representative CSV scan.
- Candidate encoding decode.
- Parquet codecs.
- Flight throughput.

```rust
#[derive(Debug, Clone)]
pub struct CalibratedCapacity {
    pub read_bytes_per_second: u64,
    pub write_bytes_per_second: u64,
    pub csv_bytes_per_second_per_core: u64,
    pub decode_bytes_per_second_per_core: u64,
    pub parquet_bytes_per_second_per_core: u64,
    pub flight_bytes_per_second: u64,
    pub safe_parser_parallelism: usize,
    pub safe_writer_parallelism: usize,
    pub target_batch_bytes: usize,
    pub target_partition_bytes: u64,
}
```

Calibration result gắn với hardware/software/config fingerprint và hết hạn khi binary/codec/storage thay đổi.

## 18. Observability

Metrics tối thiểu:

- Rows/bytes per stage.
- Cycles/byte hoặc CPU time/byte.
- Allocations/row và batch.
- Queue occupancy/backpressure time.
- Peak RSS/reserved memory.
- Spill bytes/files/time.
- Parquet codec/compression ratio.
- Row groups/pages pruned.
- Flight throughput/in-flight bytes.
- Partition p50/p95/p99.
- Straggler/speculative attempts.
- Checkpoint/commit latency.
- Controller event rate.

Span dimensions:

```text
tenant_id
pipeline_id
run_id
task_id
attempt
query_id
stage_id
partition_id
executor_id
source_segment_id
schema_version_id
```

## 19. Benchmark matrix

Datasets:

- 1M, 100M và 1B rows.
- Narrow/wide CSV.
- UTF-8, Shift-JIS, UTF-16.
- Quoted multiline fields.
- High-cardinality strings.
- Repeated/changing headers.
- Corrupt/truncated records.
- Highly skewed row widths.

Execution environments:

- Low node.
- Standard single node.
- 2/4/8+ node cluster.
- Local NVMe.
- Shared S3/R2-compatible object store.
- Network-limited configuration.

Failure injection:

- Kill parser/writer/controller.
- Disk full.
- Slow storage.
- Flight disconnect.
- Lease expiry.
- Duplicate speculative attempt.
- Slow BI/browser consumer.

## 20. Release gates

- Peak RSS nằm trong configured budget plus documented headroom.
- Không memory growth theo total row count.
- Không unbounded queue.
- Không duplicate/loss sau crash/retry.
- Controller không nhận tabular bytes.
- 1B-row benchmark hoàn tất.
- Scaling efficiency cho non-shuffle ingest:
  - 1 → 2 nodes: target ≥ 85%.
  - 1 → 4 nodes: target ≥ 80%.
  - 1 → 8 nodes: target ≥ 70%.
- Hot-path regression vượt agreed threshold làm CI/performance gate fail.
- Throughput target 100 GB/phút hoặc 1 TB/phút chỉ được công bố kèm benchmark manifest chứa hardware, input và config.

## 21. Tài liệu tham khảo kỹ thuật

- [DataFusion memory pool](https://docs.rs/datafusion/latest/datafusion/execution/memory_pool/trait.MemoryPool.html)
- [DataFusion performance and pruning](https://datafusion.apache.org/blog/output/2026/04/02/datafusion-53.0.0/)
- [DataFusion benchmark guidance](https://datafusion.apache.org/contributor-guide/testing.html)
- [Arrow Parquet parallel row-group reading](https://arrow.apache.org/rust/parquet/arrow/arrow_reader/struct.ArrowReaderBuilder.html)
- [Ballista tuning](https://datafusion.apache.org/ballista/user-guide/tuning-guide.html)
- [Tokio CPU-bound tasks](https://docs.rs/tokio/latest/tokio/)

