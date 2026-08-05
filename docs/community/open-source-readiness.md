# Open-source readiness checklist

Tài liệu này liệt kê các quyết định còn thiếu trước khi public repository và nhận external code contributions. Các mục pháp lý cần owner/tư vấn phù hợp; coding agent không được tự chốt.

## 1. Legal và identity blockers

- [ ] Chọn project name và kiểm tra trademark/name collision.
- [ ] Chọn LICENSE.
- [ ] Thêm copyright/header policy nếu cần.
- [ ] Chọn contribution mechanism: DCO, CLA hoặc repository-license model.
- [ ] Nếu dùng DCO, thêm exact DCO 1.1 process/check và sign-off guidance.
- [ ] Xác nhận dependency licenses và policy compatibility.
- [ ] Xác định ownership của benchmark datasets/fixtures.
- [ ] Xác định AI-assisted contribution disclosure/provenance policy cuối cùng.

Khuyến nghị cần owner cân nhắc, không phải quyết định đã áp dụng:

- Apache-2.0 phù hợp patent grant và hệ sinh thái Arrow/DataFusion.
- Dual MIT/Apache-2.0 phổ biến trong Rust và linh hoạt cho downstream.
- DCO thường nhẹ hơn CLA cho cộng đồng, nhưng DCO không phải software license.

## 2. Governance và contacts

- [ ] Public maintainer identities.
- [ ] CODEOWNERS/required review rules.
- [ ] Security contact/private vulnerability reporting.
- [ ] Conduct enforcement contacts.
- [ ] Release/signing owners.
- [ ] Decision/appeal process.
- [ ] Bus-factor/maintainer succession plan.

## 3. Repository configuration

- [ ] Protected default branch.
- [ ] Required CI/status checks.
- [ ] No force-push/deletion trên protected branches.
- [ ] Secret scanning và dependency update tooling.
- [ ] Private vulnerability reporting.
- [ ] Issue/PR templates.
- [ ] Discussions/roadmap policy.
- [ ] Signed release artifacts/SBOM/provenance plan.

## 4. Contribution readiness

- [x] `CONTRIBUTING.md` baseline.
- [x] `CODE_OF_CONDUCT.md` baseline.
- [x] `SECURITY.md` baseline.
- [x] `GOVERNANCE.md` baseline.
- [x] Agent instructions/handoff.
- [x] HPC performance policy.
- [x] Review checklist/ADR process.
- [ ] Public development setup works from clean clone.
- [ ] First `good-first-issue` path.
- [ ] Stable test fixtures nhỏ và legal-to-redistribute.
- [ ] CI feature matrix documented.
- [ ] English translation cho contributor-facing/architecture docs nếu community quốc tế là mục tiêu.

## 5. Release readiness

- [ ] SemVer/API compatibility policy.
- [ ] MSRV policy.
- [ ] Supported platforms.
- [ ] Reproducible `Cargo.lock`/build.
- [ ] Changelog/release notes.
- [ ] Security/advisory process.
- [ ] Performance benchmark manifest/report.
- [ ] Recovery/fault-injection release gates.
- [ ] Container/package registry ownership.

## 6. DCO note

Developer Certificate of Origin là contributor certification, không phải software license. Nếu chọn DCO, project phải dùng nguyên văn DCO 1.1 và documented sign-off workflow; không tự sửa nội dung DCO.

Tham khảo: [Linux Foundation DCO guidance](https://bestpractices.linuxfoundation.org/ip/contribution-mechanisms-dco.html).

