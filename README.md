# Rivet Data Platform

Rivet là tên mã tạm thời cho một data platform Arrow-native viết bằng Rust, hướng tới ETL/ELT, analytical compute, orchestration và data serving trong một executable duy nhất. Hệ thống được thiết kế để chạy ổn định trên node cấu hình thấp, xử lý dữ liệu lớn hơn RAM, đồng thời scale-out bằng DataFusion, Ballista và Arrow Flight.

> Trạng thái: thiết kế/khởi tạo dự án. Chưa có release production và chưa mở nhận contribution công khai cho tới khi hoàn tất quyết định LICENSE và contribution certification.

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

## Pháp lý trước khi public OSS

Repository chưa chọn LICENSE hoặc DCO/CLA. Xem [OSS readiness checklist](docs/community/open-source-readiness.md). Chủ dự án phải chốt các mục này trước khi nhận code từ external contributors.
