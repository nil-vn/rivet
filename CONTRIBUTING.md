# Hướng dẫn đóng góp

Cảm ơn bạn quan tâm tới FurrumX. Dự án đặt correctness, recoverability và hiệu năng có thể chứng minh cao hơn tốc độ merge. Một patch nhanh nhưng tạo unbounded memory, làm mơ hồ lineage hoặc không có benchmark phù hợp sẽ không được chấp nhận.

> Repository hiện phát triển private và chưa nhận external contribution. Khi mở OSS, code được cấp phép MIT và mọi external commit phải tuân thủ DCO 1.1. Xem `LICENSE`, `DCO` và `docs/community/open-source-readiness.md`.

## 1. Trước khi bắt đầu

Đọc:

- `README.md`
- `docs/README.md`
- `docs/01-product-requirements.md`
- Tài liệu domain liên quan.
- `docs/development/contributor-architecture.md`
- `docs/development/performance-quality-gates.md`
- `docs/development/review-checklist.md`

Với thay đổi lớn, mở issue/design discussion trước khi viết code:

- Public API hoặc file/wire/storage format.
- Checkpoint/resume/commit semantics.
- Dependency hoặc distributed architecture.
- New datasource/plugin runtime.
- `unsafe`.
- Performance optimization làm tăng complexity đáng kể.

## 2. Chọn issue

- Comment ý định và scope trước khi làm issue lớn.
- Không “giữ” issue vô thời hạn; cập nhật tiến độ nếu mất nhiều ngày.
- Maintainer có thể chia issue để giảm review risk.
- Security vulnerability không được thảo luận trong public issue; xem `SECURITY.md`.

Labels dự kiến:

```text
good-first-issue
help-wanted
area-ingestion
area-scheduler
area-storage
area-serving
area-distributed
performance
correctness
security
design-needed
breaking-change
```

## 3. Development workflow

1. Fork/branch từ current default branch.
2. Chọn owning module, direct dependencies và port boundary theo contributor architecture guide.
3. Giữ patch focused; tách refactor cơ học khỏi semantic/performance change khi có thể.
4. Viết test tái hiện bug trước fix.
5. Viết benchmark trước optimization nếu chưa có benchmark đại diện.
6. Cập nhật docs/ADR/config examples.
7. Chạy quality gates.
8. Self-review bằng review checklist.
9. Mở PR với evidence đầy đủ.

Không include generated files/binaries/benchmark outputs lớn trừ khi repository policy chỉ định location/format.

## 4. Coding standards

### 4.1 Rust

- Idiomatic Rust và Rust API Guidelines.
- English identifiers/comments/public docs/error codes.
- Explicit units trong names/types.
- Newtypes cho IDs, offsets, generations và fencing tokens.
- Avoid `unwrap`/panic trên untrusted input path.
- No blocking locks across `.await`.
- Bounded channels/caches/concurrency.
- No row-at-a-time object model trong data path.
- `unsafe` cần ADR, safety comment, tests và approval.
- Public API/documentation có examples và error semantics.

### 4.2 Data correctness

- Không silent coercion/drop/fallback.
- Preserve raw/bronze value khi typed conversion thất bại.
- Checkpoint sau artifact commit.
- Retry phải idempotent hoặc được fencing.
- Source/config/code changes phải invalidate resume contract đúng cách.
- Persistent counters/offsets phải xử lý dataset lớn hơn 32-bit.

### 4.3 Async/concurrency

- Tokio cho async I/O/lifecycle, không dùng cho unlimited CPU loops.
- CPU-heavy work nằm trong DataFusion hoặc fixed-size compute pool.
- Mọi queue có byte limit và cancellation path.
- Task shutdown/drop không được làm mất committed-state invariant.
- Race tests phải bao phủ lease expiry, duplicate attempt và cancellation.

### 4.4 Dependencies

PR thêm dependency phải giải thích:

- Capability thiếu hiện tại.
- Vì sao không dùng std/existing dependency.
- License và maintenance status.
- Security history/supply-chain impact.
- Binary size, compile time và feature impact.
- Compatibility với Arrow/DataFusion/Ballista BOM.

## 5. HPC contribution policy

Performance change phải tuân thủ `docs/development/performance-quality-gates.md`.

PR phải có:

- Bottleneck/profile evidence.
- Baseline và candidate commit IDs.
- Hardware/OS/Rust/build config.
- Dataset generator/hash/encoding/shape.
- Warm-up, iterations và statistics.
- Throughput, CPU, peak RSS/allocation, output size/compression ratio khi liên quan.
- Correctness equivalence.
- Complexity trade-off.

Không chấp nhận:

- Chỉ đưa một best run.
- So sánh debug với release.
- So sánh khác hardware/input/config.
- Tuyên bố GB/min/TB/min thiếu benchmark manifest.
- Optimization làm unbounded memory hoặc phá resume semantics.
- Microbenchmark improvement không có đường dẫn tới real workload khi complexity tăng lớn.

## 6. Testing gates

Baseline khi Cargo project tồn tại:

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Ngoài ra theo scope:

- Property tests cho chunk boundaries/partition ownership.
- Fuzz tests cho parser/metadata/wire inputs.
- Fault injection cho commit/recovery.
- Miri/sanitizers cho unsafe/concurrency-sensitive code.
- Feature matrix cho Ballista/Python/Wasm/S3.
- End-to-end 1B-row/performance runners cho release gates, không nhất thiết mỗi PR CI.

## 7. Documentation

PR thay behavior phải cập nhật:

- Requirement/architecture domain doc.
- Config/API example.
- Migration/compatibility note nếu breaking.
- `CHANGELOG.md` phần `Unreleased` khi user-visible.
- ADR nếu thuộc nhóm quyết định bắt buộc.

Code comments giải thích “why/invariant”, không diễn giải lại syntax.

## 8. Pull request expectations

PR description phải nêu:

- Problem và scope.
- Solution/design.
- Correctness/recovery impact.
- Performance/memory impact.
- Security impact.
- Compatibility/migration.
- Tests/benchmarks.
- Known limitations/follow-ups.
- AI assistance disclosure nếu dùng coding agent đáng kể; human contributor vẫn chịu trách nhiệm toàn bộ patch.

Giữ PR đủ nhỏ để review chính xác. Large changes nên chia theo benchmark/test, refactor, implementation và integration.

## 9. Review và merge

- Tác giả không tự merge nếu chưa có required approval/checks.
- Correctness/security/performance concerns có thể block merge ngay cả khi API hoạt động.
- Reviewer có thể yêu cầu benchmark hoặc failure test bổ sung.
- Maintainer quyết định khi improvement có đủ lớn để biện minh complexity.
- Breaking changes cần ADR và release/migration plan.

## 10. Commit history

- Commit message ngắn, imperative và giải thích intent.
- Không squash evidence cần thiết trước review; merge strategy do maintainer quyết định.
- Commit nội bộ trong private phase không bắt buộc DCO sign-off.
- Khi external contribution được mở, contributor phải đọc `DCO` và ký từng commit bằng `git commit -s`.
- `Signed-off-by` xác nhận provenance theo DCO; nó không thay thế MIT License và không phải CLA.
- Không dùng tên/email của người khác trong sign-off.

## 11. Communication

- Tôn trọng `CODE_OF_CONDUCT.md`.
- Critique code/design/evidence, không công kích cá nhân.
- English hoặc Vietnamese đều được trong giai đoạn đầu; English được khuyến nghị cho public OSS discussions.
- Benchmark disagreement được giải quyết bằng reproducible experiment, không bằng thẩm quyền hoặc cảm nhận.
