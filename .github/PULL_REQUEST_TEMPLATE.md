## Vấn đề và phạm vi

Mô tả problem, requirement/issue liên quan và explicit non-goals.

## Giải pháp

Mô tả design/implementation. Link ADR nếu có.

## Architecture fit

- Owning module và work package.
- Direct module dependencies mới hoặc thay đổi.
- Port/receipt đi qua boundary; giải thích exception so với contributor architecture guide nếu có.

## Correctness và recovery

- Ảnh hưởng tới history/checkpoint/resume/artifact/manifest/lease.
- Cách chứng minh không loss/duplicate/corruption.

## Performance và memory

- Change classification: `P0-HOT`, `P1-SCALE`, `P2-LATENCY` hoặc `P3-NEUTRAL`.
- Bottleneck/evidence.
- Baseline/candidate results và benchmark manifest link.
- Peak RSS/allocation/output-size trade-offs.

## Security và compatibility

- Trust/input/auth/secret/dependency impact.
- API/storage/wire/config compatibility và migration.

## Verification

Liệt kê command thực sự đã chạy và kết quả.

```text
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Chưa kiểm chứng

Liệt kê test/feature/platform chưa chạy và lý do.

## Checklist

- [ ] Tôi đã đọc `CONTRIBUTING.md` và tài liệu domain liên quan.
- [ ] Owning module, dependency direction và visibility tuân thủ contributor architecture guide hoặc có ADR/exception được review.
- [ ] Patch focused và không chứa unrelated changes.
- [ ] Tests tương xứng correctness/security risk.
- [ ] Queue/cache/concurrency mới đều bounded.
- [ ] User-visible behavior/config/docs/changelog đã cập nhật.
- [ ] Performance claims tuân thủ HPC policy.
- [ ] Dependency/unsafe/breaking decision đã có review/ADR phù hợp.
- [ ] Tôi đã self-review theo `docs/development/review-checklist.md`.
- [ ] Tôi disclose AI assistance đáng kể và chịu trách nhiệm toàn bộ patch.
- [ ] Nếu đây là external contribution, mọi commit đã có DCO `Signed-off-by` hợp lệ.

## AI assistance disclosure

Nêu tool/agent đã dùng đáng kể, phạm vi tạo/sửa và cách human verification được thực hiện. Ghi `None` nếu không có.
