# Rivet Data Platform — bộ tài liệu dự án

Rivet là tên mã tạm thời cho một nền tảng ETL/ELT, phân tích và data serving viết bằng Rust, biên dịch thành một executable duy nhất. Hệ thống kết hợp orchestration, columnar compute, data exchange và serving trong cùng một kiến trúc Arrow-native; chạy được trên một máy cấu hình thấp nhưng có thể scale-out trên nhiều node.

Tài liệu này tổng hợp các requirement, quyết định kiến trúc và đề xuất kỹ thuật đã thống nhất. Mọi con số hiệu năng là mục tiêu cần benchmark theo một hardware envelope cụ thể, không phải cam kết tách rời giới hạn CPU, disk, network và object storage.

## Mục lục

1. [Yêu cầu sản phẩm và tiêu chí chấp nhận](01-product-requirements.md)
2. [Kiến trúc hệ thống](02-system-architecture.md)
3. [History, lineage và exact resume](03-history-resume-lineage.md)
4. [CSV ingestion, schema discovery và schema drift](04-csv-ingestion.md)
5. [Hiệu năng, khả năng chịu tải và scale-out](05-performance-scalability.md)
6. [Extensibility, serving, vận hành và bảo mật](06-extensibility-serving-security.md)
7. [Cấu trúc dự án, dependency, lộ trình và verification](07-project-structure-delivery.md)

## Phát triển và cộng đồng

- [HPC performance và quality gates](development/performance-quality-gates.md)
- [Review checklist](development/review-checklist.md)
- [ADR process](development/adr-process.md)
- [Agent collaboration và handoff](development/agent-handoff.md)
- [Architecture Decision Records](decisions/README.md)
- [Open-source readiness](community/open-source-readiness.md)
- [Contribution guide](../CONTRIBUTING.md)
- [Security policy](../SECURITY.md)
- [Governance](../GOVERNANCE.md)
- [Code of Conduct](../CODE_OF_CONDUCT.md)

## Các quyết định cốt lõi

- Apache Arrow `RecordBatch` là data contract nội bộ.
- Apache DataFusion là compute engine; Ballista là lựa chọn ưu tiên cho distributed SQL execution.
- DAG nghiệp vụ và DAG physical query là hai tầng khác nhau.
- Tokio điều phối I/O và lifecycle; CPU-bound work phải nằm trong DataFusion hoặc bounded CPU pool.
- Zero-copy chỉ được tuyên bố tại boundary thực sự cho phép; các boundary còn lại dùng minimal-copy.
- History là append-only; current state chỉ là materialized view.
- Resume chỉ xảy ra tại safe record boundary và sau committed artifact.
- ETL dùng bronze-first: land dữ liệu không mất mát trước, ép schema nghiệp vụ sau.
- Input phải được snapshot/fingerprint trước khi có thể exact resume.
- Mọi queue và memory consumer đều có budget; kích thước dataset không được quyết định RSS.
- Controller không nằm trên data path trong distributed mode.
- Cùng một binary có thể chạy vai trò `all`, `controller`, `executor`, `serve` hoặc `run`.

## Trạng thái tài liệu

- Mức độ: architectural baseline cho thiết kế và triển khai MVP.
- Ngày chốt dependency baseline: 2026-08-05.
- Tên sản phẩm, API public và format pipeline vẫn có thể thay đổi qua ADR.
- Các quyết định làm thay đổi correctness, compatibility hoặc storage format phải được version hóa, không chỉnh âm thầm.

## Quy ước requirement

- `FR-*`: functional requirement.
- `NFR-*`: non-functional requirement.
- `INV-*`: invariant không được vi phạm.
- `AC-*`: acceptance criterion dùng để nghiệm thu.
- `ADR-*`: quyết định kiến trúc cần ghi nhận khi triển khai.
