# Audit scaffold kiến trúc

- Audit date: 2026-08-05
- Scope: repository scaffold tại `WP-001`/`WP-010`
- Classification: `P3-NEUTRAL` documentation and architecture review
- Sources: `docs/01-*` tới `docs/08-*`, `CONTRIBUTING.md`, accepted ADR-0001 tới ADR-0003 và current Rust/Cargo/CI scaffold

## 1. Kết luận

Scaffold hiện tại **phù hợp có điều kiện với vai trò bootstrap `WP-001`**, nhưng **chưa đủ làm khung triển khai an toàn cho nhiều contributor**.

Các top-level module phản ánh đúng component names trong kiến trúc và Cargo feature graph giữ đúng BOM chính. Tuy nhiên hầu hết module chỉ có một dòng module documentation; chưa có owned contracts, ports, dependency direction hoặc composition wiring. Vì vậy không thể suy ra implementation đã thỏa các `FR-*`/`INV-*`, và repository chưa ngăn contributor tạo coupling sai khi bắt đầu feature code.

Không phát hiện implementation nào hiện vi phạm checkpoint/resume/data-path invariant, vì các path đó chưa tồn tại. Đây là trạng thái "not implemented", không phải evidence "compliant".

## 2. Những phần đang phù hợp

| Area | Evidence hiện tại | Đánh giá |
|---|---|---|
| Modular monolith | Một package và một library/binary; top-level modules khớp DAG/history/checkpoint/compute/discovery/transport/storage/plugins/serving/control | Phù hợp bootstrap |
| Composition entrypoint | `main.rs` chỉ parse CLI, init telemetry và map exit code | Đúng hướng, chưa có runtime graph |
| Feature isolation | Default `local`; Flight SQL, distributed, Wasm, Python và S3 là explicit features | Phù hợp ADR-0001 |
| Dependency universe | Lockfile/tree resolve Arrow/Parquet/Flight 58.4.x, DataFusion 53.1.x, Ballista 53.0.x, object_store 0.13.2 | Không thấy incompatible Arrow major trong inspected profile |
| Rust safety baseline | Crate `forbid(unsafe_code)` và Cargo lints deny panic/unwrap/todo/unimplemented | Phù hợp baseline |
| Distributed spike scope | Ballista loopback smoke test kiểm tra `DistributedQueryExec`; docs không gọi đó là multi-node qualification | Claim được giới hạn đúng |
| Contributor process | PR template có correctness, performance, security, verification và AI disclosure | Nền process tốt |

## 3. Khoảng trống ưu tiên cao

### A-01 — Chưa có module contract và dependency direction

Mọi domain module ngoài CLI/runtime hiện là namespace rỗng. `lib.rs` export công khai tất cả namespace nhưng không quy định module nào sở hữu type/trait/service hoặc import nào bị cấm.

**Risk:** contributor dễ tạo circular dependency, concrete-adapter coupling hoặc đưa controller vào data path.

**Resolution trong audit:** thêm `contributor-architecture.md` với allowlist dependency, module ownership, workflow boundary và placement table.

### A-02 — Commit coordinator chưa có owner rõ trong scaffold

Artifact write thuộc storage, event/state thuộc history và checkpoint thuộc checkpoint. Nếu một trong ba adapter tự điều phối toàn transaction, các module sẽ import vòng hoặc invariant `INV-001`/`INV-003` bị phân tán.

**Resolution:** ADR-0003 đã chốt storage trả typed durable artifact receipt; `control::commit` điều phối một `CommitArtifactCheckpoint` metadata command, sau đó manifest prepare/seal/pointer-CAS/confirm. Implementation phải theo protocol này; không ghép ba store call độc lập rồi gọi đó là transaction.

### A-03 — Generic ingestion lifecycle chưa có application owner

Docs cấm hard-code snapshot/lease/history/checkpoint/quarantine/commit vào `CsvPlugin`, nhưng scaffold không có `ingestion` module và chưa nói ai orchestration chuỗi đó.

**Required direction:** ban đầu `control::executor` sở hữu application flow; `plugins::csv` chỉ làm format-specific work. Chỉ tách top-level `ingestion` sau design review nếu module thực sự cần lifecycle độc lập.

### A-04 — Public surface đang rộng hơn maturity

`lib.rs` dùng `pub mod` cho mọi implementation namespace. Package chưa publish, nhưng contributor có thể hiểu nhầm đây là public compatibility contract.

**Required direction:** `WP-100` phải curate public API; dùng `pub(crate)` mặc định và chỉ re-export accepted contracts. Đổi public API khi đã có consumer phải theo ADR/compatibility policy.

### A-05 — Local runtime và feature wiring chưa khép kín

Architecture nói single-node dùng Tokio cho I/O/lifecycle, nhưng direct Tokio dependency hiện chỉ được bật bởi `distributed`. Điều này chưa làm scaffold hiện tại sai vì local runtime chưa có async work, nhưng sẽ block hoặc tạo ad-hoc runtime khi implement local scheduler/storage.

**Required direction:** chốt runtime ownership và feature mapping trước `WP-110`/`WP-120` implementation; cập nhật ADR-0001 hoặc superseding ADR nếu supported profile contract thay đổi.

### A-06 — ADR baseline chưa hoàn tất

ADR-0001 tới ADR-0003 đã accepted, bao gồm event state và artifact/checkpoint/manifest ordering/local durability. Development plan vẫn yêu cầu memory permits và Bronze semantics trước khi implementation tương ứng ổn định.

**Risk:** contributor sẽ encode durable format và correctness contract trong code-only PR.

**Required direction:** ưu tiên ADR-0004 trước hot-path parallel implementation; mọi persistence work phải trace tới ADR-0002/0003.

## 4. Khoảng trống delivery/enforcement

| Gap | Trạng thái hiện tại | Gate/work package phù hợp |
|---|---|---|
| Không có architecture dependency check | Chỉ có Rust lint và docs check | `WP-030`; bắt đầu bằng review allowlist, tự động hóa khi imports xuất hiện |
| Không có fixture/benchmark/fault harness | Không có `benches/`, generator hoặc fault-injection directories | `WP-030`, bắt buộc trước P0 implementation |
| Không có migrations/proto/WIT | `build.rs` placeholder, chưa có durable/wire artifacts | Đúng nếu chưa bắt đầu `WP-200`/`WP-640`/`WP-700` |
| Feature jobs chủ yếu chỉ compile | Non-distributed feature profiles chưa có runtime compatibility tests | Bổ sung theo từng feature implementation; không claim supported runtime sớm |
| Không có end-to-end/fault/property tests | Chỉ ba CLI unit tests và một distributed loopback smoke test | Expected tại bootstrap, không đủ cho `G1` trở lên |
| Không có invariant-to-code enforcement | Traceability hiện mới nằm trong plan | Mỗi WP phải thêm runtime/type constraint và test owner theo invariant map |
| Chưa có example pipeline/config parser | Docs có example nhưng CLI chỉ có `doctor` | Expected trước `WP-210`/`WP-430`; docs phải tiếp tục ghi là target |

## 5. Các module không cần thêm ngay

Không nên tạo top-level module rỗng chỉ vì capability cross-cutting:

- `ingestion`: application flow đặt tại `control::executor`, format code tại `plugins`, generic decisions tại `discovery`.
- `security`: protocol-neutral identity/policy contract có thể nằm shared kernel/plugin contract; middleware ở `serving` và `control` cho tới khi ADR chứng minh cần boundary riêng.
- `catalog`: MVP manifest thuộc `storage`, provider thuộc `compute`; catalog plugin là phase sau.
- `observability`: mỗi module emit metrics/spans, `runtime` cài subscriber/exporter.

Tách các module/crate này sớm sẽ tạo abstraction chưa có evidence và không tự giải quyết ownership. Nếu code growth hoặc team ownership chứng minh boundary ổn định, dùng ADR để tách.

## 6. Thứ tự hành động đề xuất

1. Merge contributor architecture guide và dùng nó trong issue/PR review.
2. Hoàn tất ADR-0004 memory permit; event state và commit ordering đã được chốt trong ADR-0002/0003.
3. Thực hiện `WP-100`: durable IDs, shared receipts, canonical hashes, stable errors và curated visibility.
4. Thực hiện `WP-030`: fixtures, benchmark manifest, fault hooks và architecture enforcement phù hợp actual imports.
5. Chỉ sau đó mở lane song song cho resource/transport, history/DAG và snapshot/storage theo dependency graph của development plan.
6. Xây vertical slice nhỏ đi hết snapshot → bounded batch → immutable artifact → checkpoint/history → inspection trước khi tối ưu hoặc distributed customization.

## 7. Evidence đã chạy

Các kiểm tra read-only trong audit:

```text
cargo metadata --locked --no-default-features --no-deps --format-version 1
cargo tree --locked -d --depth 2
cargo tree --locked --features distributed -d --depth 2
cargo test --locked --workspace -- --list
cargo fmt --all --check
cargo check --locked --no-default-features --workspace --all-targets
cargo check --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
./scripts/check-docs.sh
```

Kết quả:

- Metadata và cả hai dependency-tree commands thành công bằng Cargo/Rust 1.97.1.
- Format, minimal/default check, Clippy với warnings denied, default tests và documentation checks đều pass.
- Default test profile chạy ba CLI unit tests; cả ba pass.
- Distributed smoke test bị compile-out trong default profile; audit không chạy distributed test vì environment không có `protoc`.
- Không thực hiện benchmark; audit là `P3-NEUTRAL` và repository chưa có benchmark harness.

## 8. Remaining risks

- Audit không chứng minh correctness, recovery, bounded memory hoặc distributed behavior của capability chưa được implement.
- Dependency allowlist mới là review policy; chưa có automated architecture test.
- Durable contract placement vẫn phụ thuộc các ADR chưa accepted.
- Public module visibility và local Tokio feature mapping chưa được sửa trong code vì đều chạm API/build architecture cần maintainer chốt scope.
