# Security Policy

## 1. Báo cáo vulnerability

Không mở public issue cho vulnerability chưa được vá.

Khi repository được host trên GitHub, dùng **Security → Report a vulnerability** nếu private vulnerability reporting đã bật. Nếu chưa bật, liên hệ project maintainers qua private channel được liệt kê trong repository/organization profile. Không gửi exploit, secret hoặc sensitive dataset qua public discussion.

Repository hiện ở giai đoạn pre-release và chưa công bố security contact/SLA chính thức. Đây là mục bắt buộc trong `docs/community/open-source-readiness.md` trước public launch.

## 2. Thông tin cần cung cấp

- Affected commit/version/feature.
- Deployment assumptions.
- Reproduction steps hoặc minimal PoC.
- Impact: confidentiality, integrity, availability.
- Data/source formats liên quan.
- Workaround nếu biết.
- Disclosure constraints/timeline mong muốn.

## 3. Phạm vi ưu tiên

- CSV/Parquet/Arrow IPC parser memory safety hoặc resource exhaustion.
- Path traversal, symlink escape, SSRF và arbitrary local URI access.
- Flight/gRPC/REST authentication/authorization bypass.
- Signed ticket replay/forgery.
- Tenant data leakage.
- Source snapshot/manifest/checkpoint tampering.
- Wasm sandbox escape/capability leak.
- Python/native plugin trust boundary.
- Secret/log leakage.
- Dependency/supply-chain compromise.

## 4. Supported versions

Trước release 1.0, chỉ current default branch và release candidate mới nhất được xem xét. Khi có stable releases, bảng supported versions sẽ được cập nhật tại đây.

## 5. Secure development baseline

- Deny-by-default capabilities.
- TLS cho remote deployments.
- Per-request token validation.
- Input size/depth/complexity limits.
- Bounded memory/concurrency/disk.
- Fuzz malformed external formats.
- No secret/raw sensitive data in logs.
- Dependency review, lockfile, advisories và SBOM.
- `unsafe` cần isolated review và dedicated tests.
- Security-sensitive changes cần reviewer có domain expertise.

## 6. Disclosure process

Maintainers sẽ cố gắng:

1. Xác nhận receipt bằng private channel.
2. Triage impact và affected versions.
3. Phối hợp fix/test/advisory.
4. Release fix trước hoặc cùng disclosure khi khả thi.
5. Ghi nhận reporter nếu họ đồng ý.

Không hứa thời gian phản hồi cứng trước khi security team/contact được thiết lập.

