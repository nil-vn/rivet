# Hướng dẫn bắt buộc cho coding agents

File này áp dụng cho toàn repository. Agent phải đọc file này và các tài liệu được liên kết trước khi sửa code hoặc tài liệu.

## 1. Nguồn sự thật

Đọc theo thứ tự:

1. `docs/01-product-requirements.md` — requirements, invariants và acceptance criteria.
2. `docs/02-system-architecture.md` — component/data-flow boundaries.
3. Tài liệu domain liên quan trong `docs/03-*` tới `docs/06-*`.
4. `CONTRIBUTING.md` — workflow và quality gates.
5. `docs/development/performance-quality-gates.md` — bắt buộc với hot path/HPC changes.
6. `docs/development/review-checklist.md` — self-review trước handoff.

Nếu code và tài liệu mâu thuẫn, không tự chọn một phía. Báo rõ mâu thuẫn, xác định requirement/ADR có thẩm quyền và sửa cả implementation lẫn tài liệu trong cùng change khi được phép.

## 2. Invariants không được vi phạm

- Checkpoint chỉ commit sau immutable output artifact.
- Không resume khi resume contract khác.
- Không silent data loss hoặc silent fallback.
- Không publish manifest tham chiếu partial/uncommitted object.
- Không buffer toàn file/query result trong normal path.
- Không tạo queue, cache hoặc concurrency không giới hạn.
- Controller không proxy tabular bytes trong distributed mode.
- Không dùng petgraph `NodeIndex` làm durable identity.
- Không tuyên bố zero-copy tại boundary có transcoding, compute allocation, Wasm hoặc network.
- Không ghi history/reject theo từng record vào control database.
- Không trộn incompatible Arrow/DataFusion/Ballista major-version universes.

Change có thể phá invariant phải dừng lại, mở ADR và yêu cầu maintainer quyết định.

## 3. Quy tắc làm việc trong shared workspace

- Kiểm tra `git status --short` và đọc diff trước khi sửa.
- Mọi thay đổi có sẵn thuộc về người dùng/agent khác; không revert hoặc format unrelated files.
- Chỉ nhận một scope cụ thể; tránh sửa cùng file với agent khác nếu chưa phối hợp.
- Không chạy destructive git commands.
- Không commit, push, tạo tag/release hoặc mở PR nếu chưa được yêu cầu rõ.
- Nếu được phép phân công agent khác, task phải bounded, có file ownership rõ và không trùng scope.
- Khi handoff, dùng template trong `docs/development/agent-handoff.md`.

## 4. Ngôn ngữ và style

- Rust identifiers, code comments, log/event/error codes và public API names dùng English.
- Tài liệu kiến trúc hiện dùng technical Vietnamese; PR/issue public có thể dùng English hoặc Vietnamese. English được khuyến nghị khi hướng tới cộng đồng quốc tế.
- Không dùng marketing claims như “zero-copy”, “exactly-once” hoặc “TB/min” nếu thiếu scope và evidence.
- Tên durable fields/types phải thể hiện units: `_bytes`, `_micros`, `_rows`.
- Persistent counters/offsets dùng `u64` trừ khi có lý do được ghi rõ.

## 5. Rust quality baseline

- Ưu tiên type system/newtypes để encode invariants.
- Public APIs tuân thủ Rust API Guidelines.
- Không `unwrap`, `expect`, `panic!`, `todo!` hoặc `unimplemented!` trên production input path, trừ invariant nội bộ đã được chứng minh và giải thích.
- Error phải có actionable context nhưng không lộ secret/raw sensitive data.
- Không giữ blocking lock qua `.await`.
- Không chạy CPU loop dài trực tiếp trong async task.
- Không tạo một task/future/allocation cho mỗi row.
- Không dùng `String`, `Vec<String>`, `HashMap<String, Value>`, regex hoặc Serde cho từng record trong hot path.
- `unsafe` mặc định bị cấm trong application modules. Exception cần safety contract, focused tests, Miri/sanitizer plan và maintainer approval.
- Dependency mới cần justification: capability, license, maintenance, security, binary size, compile time và duplicate type universe.

## 6. HPC/performance rules

Change chạm parser, decoder, Arrow builder, transport, DataFusion plan, Parquet writer, allocator, hashing, compression hoặc scheduling phải:

1. Xác định bottleneck giả thuyết.
2. Có representative benchmark trước thay đổi hoặc thêm benchmark trước implementation.
3. Đo cùng hardware/config/dataset/build profile.
4. Báo raw samples/statistics, không chỉ best run.
5. Báo throughput, CPU, peak RSS/allocation và output size/correctness liên quan.
6. Chứng minh improvement lớn hơn benchmark noise và đáng với complexity.
7. Chạy correctness/fault tests; performance không được đổi semantics âm thầm.
8. Lưu benchmark manifest theo policy.

Optimization không có profile/measurement sẽ không được coi là performance contribution hoàn chỉnh.

## 7. Correctness và recovery tests

Mọi thay đổi tới history/checkpoint/artifact/manifest/lease phải có test cho:

- Retry/idempotency.
- Crash trước và sau commit boundary.
- Stale fencing token.
- Resume contract mismatch.
- Orphan/partial artifact reconciliation.
- Không gap/overlap/duplicate visible records.

Mọi thay đổi CSV/file ingestion phải xem xét:

- Arbitrary chunk boundaries.
- Quoted multiline records.
- Encoding partial sequences.
- No/multi/repeated/changing headers.
- Invalid rows/quarantine.
- One-billion-row counters và bounded memory.

## 8. Kiểm tra bắt buộc trước handoff

Khi Cargo project đã tồn tại, chạy trong phạm vi feature phù hợp:

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Feature-specific change phải chạy feature matrix liên quan, ví dụ `distributed`, `python`, `wasm`, `s3`. Nếu môi trường thiếu external dependency, ghi rõ command chưa chạy và lý do; không nói “all tests pass”.

Thay đổi tài liệu phải kiểm tra internal links, code fences, trailing whitespace và đồng bộ mục lục.

## 9. Khi nào cần ADR

- Thay invariant/correctness semantics.
- Thay storage, event, manifest hoặc wire format.
- Thay dependency BOM/core engine.
- Thêm `unsafe` hoặc native dynamic ABI.
- Thay exactly-once/resume contract.
- Thêm external service bắt buộc.
- Chấp nhận performance regression có chủ đích.
- Thay public API hoặc compatibility policy.

Theo `docs/development/adr-process.md`; không giấu quyết định kiến trúc trong một code-only PR.

## 10. Handoff bắt buộc

Final report phải nêu:

- Outcome, không chỉ steps.
- Files đã sửa.
- Tests/benchmarks đã chạy và kết quả.
- Tests chưa chạy.
- Correctness/performance/security risks còn lại.
- ADR/requirement nào bị ảnh hưởng.
- Benchmark claims kèm environment/manifest.

## 11. Tài liệu tham khảo

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [DataFusion contributor and benchmark guidance](https://datafusion.apache.org/contributor-guide/index.html)

