# ADR-0001: Dependency BOM, MSRV và feature isolation

- Status: Accepted
- Date: 2026-08-05
- Owners: Project owner and maintainers
- Related requirements: `INV-010`, `FR-COMP-001`, `FR-COMP-002`, `FR-PLUG-004`
- Related work packages: `WP-001`, `WP-010`, `WP-020`

## Context

FurrumX dùng Arrow types xuyên ingestion, DataFusion, Parquet, Flight, Python và Ballista. Hai Arrow major universe trong cùng process sẽ tạo type/ABI conversion, duplicate code và correctness risk. Latest standalone DataFusion không tự động tương thích latest Ballista, vì hai project có release cadence khác nhau.

Wasmtime và PyO3 cũng đặt giới hạn Rust/compiler và native environment khác default local build. Nếu bật tất cả dependency mặc định, binary/build time sẽ đi ngược mục tiêu lightweight và làm WSL bootstrap khó kiểm soát.

## Decision drivers

- Một Arrow type universe trong mỗi supported feature profile.
- Ballista/DataFusion compatibility có bằng chứng từ dependency graph.
- Default local build nhỏ hơn extended profiles.
- Reproducible resolution bằng committed `Cargo.lock`.
- MSRV đủ cho toàn optional feature universe đã công bố.
- Linux/WSL là platform verification đầu tiên.

## Options considered

### Latest DataFusion independent of Ballista

Ưu điểm là nhận feature mới nhất. Nhược điểm là distributed build có DataFusion/Arrow universe khác hoặc phải duy trì adapter/fork. Option này bị từ chối cho baseline.

### Ballista-aligned BOM

Giữ DataFusion 53.x với Ballista 53.x và Arrow/Parquet/Flight 58.x. Đây là option được chọn vì distributed compatibility quan trọng hơn việc chạy latest standalone engine.

### Separate local/distributed binaries với hai BOM

Có thể cô lập type universe nhưng phá mục tiêu cùng source/binary profile và tăng test matrix. Chưa chọn; chỉ xem xét lại bằng ADR nếu Ballista release lag trở thành blocker dài hạn.

## Decision

Baseline từ committed `Cargo.lock`:

| Component | Resolved line |
|---|---:|
| Rust edition | 2024 |
| Package MSRV | 1.94 |
| Development/CI toolchain | 1.97.1 |
| Arrow/Parquet/Flight | 58.4.x |
| DataFusion | 53.1.x |
| Ballista | 53.0.x |
| object_store | 0.13.2 |
| PyO3 | 0.28.3 |
| Wasmtime/Wasmtime WASI | 47.0.x |

Rules:

- `Cargo.lock` được commit và CI dùng `--locked`.
- Không nâng DataFusion lên 54.x trong distributed feature cho tới khi Ballista-aligned upgrade spike pass.
- `default = ["local"]`; Flight SQL, S3, Wasm, Python và distributed là explicit features.
- Minimal `--no-default-features` không kéo Arrow/DataFusion/Ballista/Python/Wasmtime.
- Python profile chỉ được coi supported sau PyArrow integration spike riêng.
- Distributed dependency profile được compile trong CI và có Ballista standalone loopback smoke test; multi-process/multi-node support vẫn cần qualification riêng.
- Dependency upgrade chạm BOM cần `cargo tree` evidence và ADR update/supersession.

## Consequences

- Local code không nhận DataFusion 54 feature ngay lập tức.
- `Cargo.lock` có thể resolve patch release mới trong dòng tương thích khi dependency constraints thay đổi; bảng này phải phản ánh lockfile thực tế.
- Wasmtime nâng package-wide MSRV lên 1.94 dù minimal profile có thể compile bằng compiler thấp hơn; dự án ưu tiên một MSRV dễ hiểu.
- Feature compile matrix tốn CI time nhưng ngăn hidden type universe và native-build failures.

## Verification

Đã pass trên WSL2 x86_64 với Rust 1.97.1:

```text
cargo check --locked --no-default-features
cargo check --locked --workspace --all-targets
cargo check --locked --features flight-sql
cargo check --locked --features s3
cargo check --locked --features wasm
cargo check --locked --features python
cargo check --locked --features distributed
cargo test --locked --features distributed --test distributed_smoke
```

Chưa hoàn tất tại thời điểm ADR được chấp nhận:

```text
PyArrow C Stream round trip
Ballista multi-process remote query smoke test
```

Distributed build cần `protoc` vì Ballista scheduler kéo Substrait protocol generation. Smoke test khởi động scheduler và executor thật trong cùng process, giao tiếp qua loopback, giới hạn hai concurrent tasks, thực thi aggregate query và xác minh Arrow result. Đây chưa phải bằng chứng multi-process, multi-node, recovery hoặc throughput.

## References

- [DataFusion 53 upgrade guide](https://datafusion.apache.org/library-user-guide/upgrading/53.0.0.html)
- [Ballista 53 crate dependencies](https://docs.rs/crate/ballista/latest)
- [Wasmtime crate metadata](https://docs.rs/crate/wasmtime/latest)
