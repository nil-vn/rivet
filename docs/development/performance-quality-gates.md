# HPC performance và quality gates

## 1. Mục tiêu

Policy này áp dụng cho mọi contribution có thể ảnh hưởng throughput, latency, CPU, memory, disk/network I/O, binary size hoặc scalability. Mục tiêu là tạo improvement có thể tái lập mà không làm suy yếu correctness, recovery, security hoặc maintainability.

## 2. Phân loại change

### `P0-HOT`

Chạm critical data path:

- Encoding/UTF validation.
- Record boundary/CSV parser.
- Arrow builders/kernels.
- Local/Flight batch transport.
- Parquet read/write/compression.
- Hashing/checksum.
- Partition/shuffle/scheduler hot loop.
- Allocator/memory pool/spill.

Yêu cầu full benchmark evidence.

### `P1-SCALE`

Ảnh hưởng distributed scalability, controller metadata rate, work stealing, storage requests hoặc backpressure. Yêu cầu single-node và scale/failure evidence phù hợp.

### `P2-LATENCY`

Ảnh hưởng query planning, Flight/REST latency, startup hoặc plugin invocation. Yêu cầu latency distribution và resource impact.

### `P3-NEUTRAL`

Docs/test/refactor không dự kiến ảnh hưởng performance. Nếu refactor nằm trên hot path, vẫn cần smoke benchmark để chứng minh neutral.

## 3. Correctness gates đứng trước performance

Không merge optimization nếu:

- Thay visible dataset semantics ngoài scope.
- Làm checkpoint đi trước artifact.
- Tạo gap/overlap/duplicate sau retry.
- Silent drop/coerce/fallback input.
- Tạo unbounded queue/cache/concurrency.
- Bỏ authentication/resource limit.
- Dựa vào undefined behavior hoặc unsafe contract chưa chứng minh.

Performance benchmark chỉ hợp lệ khi baseline và candidate tạo kết quả tương đương theo declared semantics.

## 4. Benchmark workflow

1. Xác định workload/bottleneck bằng profile hoặc metrics.
2. Thêm/chọn benchmark đại diện trước khi optimization.
3. Chạy baseline từ clean release build.
4. Áp thay đổi focused.
5. Chạy candidate cùng environment.
6. Lặp đủ để ước lượng noise/distribution.
7. Kiểm tra correctness output/hash/row count.
8. Profile candidate để xác nhận improvement đến từ hypothesized path.
9. Báo trade-offs và raw data/manifest.

DataFusion cũng yêu cầu performance PR có benchmark và khuyến nghị benchmark baseline riêng; FurrumX áp cùng tinh thần cho parser/ingestion/distributed paths.

## 5. Environment control

Benchmark report phải ghi:

- CPU model, core/thread count, NUMA.
- RAM và memory limit.
- Disk/filesystem/mount options.
- Network/link/object-store topology.
- OS/kernel/container/cgroup.
- Rust toolchain và target flags.
- Commit SHA và Cargo.lock hash.
- Feature flags/release profile.
- Dataset generator/version/hash.
- Encoding, row/column shape, delimiter/header profile.
- Parquet codec/row-group/file target.
- Warm-up, iterations, concurrency.
- Background load/power governor nếu biết.

Baseline và candidate phải cùng máy/config. Cross-machine results chỉ dùng capacity comparison, không dùng để chứng minh code improvement.

## 6. Metrics bắt buộc

### Ingestion

- Raw input bytes/s.
- Decoded bytes/s.
- Records/s.
- Arrow output bytes/s.
- Committed Parquet bytes/s.
- CPU time hoặc cycles/byte.
- Peak RSS và accounted memory.
- Allocations/record hoặc allocation bytes.
- Output size/compression ratio.
- Reject/correctness counts.

### Query/serving

- p50/p95/p99 latency.
- Throughput/QPS hoặc rows/s.
- Planning vs execution time.
- CPU/peak RSS/spill.
- Result bytes and cancellation behavior.

### Distributed

- Per-node và aggregate throughput.
- Scaling efficiency.
- Network/shuffle bytes.
- Storage request rate.
- Partition p50/p95/p99/skew.
- Controller CPU/event/lease rate.
- Recovery/reprocessing after node loss.

## 7. Statistical policy

- Không dùng một best run.
- Giữ warm-up tách khỏi samples.
- Dùng Criterion hoặc harness xuất distribution/raw samples.
- Improvement phải lớn hơn noise của runner.
- Trên controlled performance runner, statistically credible regression trên critical metric phải được điều tra.
- Mặc định, regression >5% trên critical throughput/latency hoặc >10% peak RSS sẽ block merge trừ khi có approved ADR/risk acceptance; suite-specific thresholds có thể chặt hơn.
- Improvement rất nhỏ không biện minh complexity/unsafe/dependency đáng kể.

Không áp threshold tự động trên noisy shared CI mà không có confidence/control; dùng dedicated runner hoặc manual rerun.

## 8. Memory quality gates

Hard failures:

- RSS tăng theo total input rows sau steady state.
- Queue/cache không có byte cap.
- Per-record heap object trong documented hot path mà không có evidence.
- Writer/parser concurrency không có admission.
- Low-memory profile OOM trong required test.
- Spill/temp disk không có quota/cleanup.
- Allocation/file growth xảy ra trước byte-accounted permit, hoặc queue chỉ bound theo item count.
- DataFusion dùng unbounded production pool hoặc cùng physical bytes bị charge lại ở global pool.
- Cancellation/restart leak permit, hoặc disk capacity được trả trước verified delete/durable ownership transfer.

Benchmark memory ở ít nhất hai dataset sizes để phát hiện slope, ví dụ 10M và 100M rows. Với 1B-row release test, RSS phải ổn định quanh configured envelope, không chỉ “chưa OOM”.

Report phải tách configured capacity, admitted task envelopes, physical charged bytes, channel in-flight credits, DataFusion internal reservations, safety headroom, peak RSS/RSS delta và temp recovery debt. Channel credit không phải physical allocation và không được cộng vào RSS estimate; DataFusion child reservation không acquire lại global physical pool.

## 9. CPU and allocation review

Contributor phải xem xét:

- Copies và buffer lifetime.
- Branching/validation trong inner loop.
- UTF/quote/delimiter scanning.
- Per-cell dynamic dispatch.
- Hash map/hash algorithm và DoS implications.
- String formatting/logging trong hot path.
- Arc/refcount clones.
- Lock contention/false sharing.
- Task scheduling/async wakeups.
- SIMD portability/CPU feature detection.

Không dùng unsafe/SIMD intrinsics nếu Arrow/std/existing kernel đã đủ và benchmark không chứng minh lợi ích.

## 10. I/O and storage review

- Sequential/range/vectored access pattern.
- Same-disk read+write contention.
- mmap safety với mutable file.
- Object-store request amplification.
- Multipart concurrency/memory.
- Small-file count.
- Parquet row-group/pruning impact.
- Compression CPU vs network/storage trade-off.
- fsync/durability cost không được bỏ khỏi correctness benchmark một cách âm thầm.

Benchmark `unsafe_fast_mode` không được dùng đại diện default durable mode.

## 11. Distributed scaling gate

Non-shuffle ingest target ban đầu:

```text
1 → 2 nodes: ≥ 85% scaling efficiency
1 → 4 nodes: ≥ 80%
1 → 8 nodes: ≥ 70%
```

Nếu không đạt, report phải phân tích source/sink/network/controller/skew bottleneck. Không “sửa” bằng cách giảm durability/history granularity ngoài policy.

TB/phút claim phải kèm aggregate bandwidth proof và benchmark manifest. Controller không được nhận tabular payload trong benchmark topology.

## 12. Benchmark manifest

Mỗi published performance result cần manifest tương đương:

```toml
[benchmark]
name = "csv_utf8_to_parquet"
baseline_commit = "<sha>"
candidate_commit = "<sha>"
date = "<iso-date>"

[build]
rustc = "<version>"
profile = "release"
features = ["local"]
rustflags = "<flags>"
cargo_lock_hash = "<hash>"

[hardware]
cpu = "<model>"
physical_cores = 0
logical_cores = 0
memory_bytes = 0
disk = "<model-and-filesystem>"
network = "<topology>"

[dataset]
generator = "<name-version>"
content_hash = "<hash>"
raw_bytes = 0
rows = 0
columns = 0
encoding = "utf-8"
header_profile = "single-row"

[configuration]
batch_target_bytes = 0
partition_target_bytes = 0
parquet_codec = "snappy"
parser_workers = 0
writer_workers = 0

[method]
warmup_runs = 0
measurement_runs = 0
```

Raw results có thể lưu dưới CI artifact hoặc approved benchmark-results location; không commit dataset/output lớn tùy tiện.

## 13. Complexity budget

PR phải trả lời:

- Bao nhiêu performance improvement?
- Workload thực nào hưởng lợi?
- Code/dependency/unsafe complexity tăng bao nhiêu?
- Maintenance/portability/debug cost?
- Có làm path khác chậm/tốn RAM hơn không?
- Có feature flag/fallback portable path không?

Maintainer có thể từ chối optimization thật sự nhanh hơn nếu lợi ích nhỏ hơn long-term complexity/risk.

## 14. Review evidence checklist

- [ ] Workload đại diện và bottleneck được xác định.
- [ ] Baseline/candidate cùng environment.
- [ ] Raw samples/statistics có sẵn.
- [ ] Correctness output tương đương.
- [ ] CPU/RSS/allocation/output-size được báo.
- [ ] Envelope/physical/flow-credit/DataFusion/headroom/temp-debt metrics được tách, không double-account.
- [ ] Permit ownership, acquire-before-growth, progress reserve và cancellation/drop paths có evidence.
- [ ] Temp files/restart debt giữ charge tới verified delete hoặc durable-capacity transfer.
- [ ] Low-resource effect được xem xét.
- [ ] Distributed/storage side effects được xem xét.
- [ ] Complexity và portability được giải thích.
- [ ] Docs/changelog/benchmark manifest cập nhật.

## 15. Tham khảo

- [Rust Performance Book — Benchmarking](https://nnethercote.github.io/perf-book/benchmarking.html)
- [DataFusion contributor performance guidance](https://datafusion.apache.org/contributor-guide/index.html)
- [DataFusion benchmark baselines](https://datafusion.apache.org/contributor-guide/testing.html)
