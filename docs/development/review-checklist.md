# Review checklist

Checklist này dùng cho author self-review, human reviewer và coding agent handoff. Không phải mọi mục đều áp dụng; đánh dấu `N/A` kèm lý do khi cần.

## Scope và design

- [ ] PR giải quyết một problem rõ ràng, không trộn unrelated refactor.
- [ ] Requirement IDs/domain docs liên quan được chỉ ra.
- [ ] ADR được thêm nếu thay invariant/API/storage/wire/dependency architecture.
- [ ] Backward compatibility/migration được mô tả.
- [ ] Dependency mới có justification/license/security/size analysis.

## Correctness

- [ ] Không silent loss/drop/coercion/fallback.
- [ ] Retry/resume/idempotency semantics được giữ.
- [ ] Checkpoint chỉ sau committed artifact.
- [ ] Fencing/stale attempt được xử lý.
- [ ] Persistent counters/offsets không overflow ở Big Data scale.
- [ ] Cancellation/error/partial input paths có test.
- [ ] Determinism/cache semantics được khai báo.

## Async và concurrency

- [ ] Không blocking operation/lock qua `.await`.
- [ ] CPU-bound work không chiếm Tokio async workers vô hạn.
- [ ] Queue/concurrency/cache có hard bound.
- [ ] Buffer/file growth acquire byte-accounted lease trước use; item count không thay byte cap.
- [ ] Backpressure đi xuyên toàn pipeline.
- [ ] Acquire order/progress reserve ngăn hold-and-wait deadlock.
- [ ] Race, shutdown và lease expiry được kiểm thử.
- [ ] Lock order/contention/false sharing được xem xét.

## Memory và HPC

- [ ] Không per-row heap object/message/future trong hot path.
- [ ] Buffer copies/lifetimes được phân tích.
- [ ] Envelope reservation, physical charge, channel credit và DataFusion reservation được báo riêng, không double-account.
- [ ] Cancellation/view/ownership transfer trả physical lease đúng một lần khi allocation cuối drop.
- [ ] Peak RSS/allocation impact được đo.
- [ ] Low-memory profile được xem xét.
- [ ] Benchmark tuân thủ performance policy.
- [ ] Improvement lớn hơn noise và xứng đáng complexity.
- [ ] Không có unsupported zero-copy/GB-min claim.

## Data formats và storage

- [ ] Chunk/record/encoding boundaries an toàn.
- [ ] Malformed/corrupt input có resource limits.
- [ ] Schema/header drift được audit.
- [ ] Parquet file hoàn chỉnh trước visible commit.
- [ ] Local/object-store atomicity assumptions đúng.
- [ ] Orphan/staging cleanup có reconciliation.
- [ ] Temp-disk charge chỉ trả sau verified delete hoặc durable-capacity ownership transfer; restart debt được account.
- [ ] No small-file/request amplification bất hợp lý.

## Security/privacy

- [ ] Auth/authz kiểm tra ở đúng boundary.
- [ ] Secret/raw sensitive data không vào logs/errors/history.
- [ ] Paths/URIs được canonicalize/allowlist.
- [ ] External payload size/depth/time limits.
- [ ] Wasm/Python/native trust boundary đúng.
- [ ] `unsafe` có safety contract và tests.

## API và Rust quality

- [ ] Naming/types theo Rust API Guidelines.
- [ ] Public types implement/document common traits phù hợp.
- [ ] Errors actionable và non-sensitive.
- [ ] Không production `unwrap`/panic trên untrusted inputs.
- [ ] Public API docs/examples được cập nhật.
- [ ] Feature flags không tạo incompatible type universe.

## Tests và docs

- [ ] Unit/integration/property/fuzz/fault test tương xứng risk.
- [ ] Formatting, check, clippy và test commands đã chạy.
- [ ] Feature matrix liên quan đã chạy hoặc limitation được ghi rõ.
- [ ] Docs/config examples/changelog cập nhật.
- [ ] Internal links/code fences/trailing whitespace sạch.

## Handoff/merge

- [ ] Known limitations và follow-ups được nêu.
- [ ] AI assistance được disclose khi đáng kể.
- [ ] Không chứa unrelated user/agent changes.
- [ ] Required approvals/checks hoàn tất.
