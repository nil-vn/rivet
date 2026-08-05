# FurrumX Data Platform

FurrumX là một data platform Arrow-native viết bằng Rust, hướng tới ETL/ELT, analytical compute, orchestration và data serving trong một executable duy nhất. Hệ thống được thiết kế để chạy ổn định trên node cấu hình thấp, xử lý dữ liệu lớn hơn RAM, đồng thời scale-out bằng DataFusion, Ballista và Arrow Flight.

> Trạng thái: private development trên Linux/WSL. Chưa có production release và chưa nhận external contribution.

## Đặc tính mục tiêu

- Streaming và bounded memory mặc định.
- Append-only task history, lineage và exact resume.
- CSV ingestion không cần biết trước encoding/header/schema.
- Lossless bronze-first landing vào Parquet.
- DataFusion columnar compute; Ballista distributed SQL.
- Arrow Flight SQL và Arrow IPC serving.
- Rust-native, PyO3 và sandboxed Wasm extensions.
- HPC quality gates dựa trên benchmark có thể tái lập, không dựa trên tuyên bố cảm tính.

## Tài liệu

- [Bộ tài liệu kiến trúc](docs/README.md)
- [Yêu cầu sản phẩm](docs/01-product-requirements.md)
- [Kiến trúc hệ thống](docs/02-system-architecture.md)
- [History và exact resume](docs/03-history-resume-lineage.md)
- [CSV ingestion](docs/04-csv-ingestion.md)
- [Performance và scalability](docs/05-performance-scalability.md)
- [Extensibility, serving và security](docs/06-extensibility-serving-security.md)
- [Cấu trúc dự án và delivery](docs/07-project-structure-delivery.md)
- [Kế hoạch phát triển chi tiết](docs/08-development-plan.md)

## Cộng tác

- Con người: đọc [CONTRIBUTING.md](CONTRIBUTING.md).
- Coding agents: đọc [AGENTS.md](AGENTS.md) trước khi thay đổi repository.
- Review: dùng [review checklist](docs/development/review-checklist.md).
- Performance change: tuân thủ [HPC performance policy](docs/development/performance-quality-gates.md).
- Security issue: đọc [SECURITY.md](SECURITY.md), không mở public issue cho vulnerability chưa vá.

## Khởi động trên WSL

```bash
./scripts/check-wsl.sh
cargo run --locked -- doctor
cargo check --locked --workspace --all-targets
cargo test --locked --workspace
```

Workspace hiện ở dưới `/mnt/c` phù hợp functional testing nhưng không phù hợp HPC benchmark. Xem [hướng dẫn Linux/WSL](docs/development/wsl-setup.md) trước khi đo I/O hoặc throughput.

## Pháp lý trước khi public OSS

FurrumX được cấp phép theo [MIT License](LICENSE). Khi mở nhận external contribution, dự án dùng [Developer Certificate of Origin 1.1](DCO) thay vì CLA; contributor xác nhận bằng `Signed-off-by`. Xem [OSS readiness checklist](docs/community/open-source-readiness.md).
