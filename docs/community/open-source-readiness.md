# Open-source readiness checklist

Tài liệu này liệt kê các quyết định còn thiếu trước khi public repository và nhận external code contributions. FurrumX hiện phát triển private. Các mục pháp lý còn lại cần owner/tư vấn phù hợp; coding agent không được tự thay đổi.

## 1. Legal và identity blockers

- [x] Chọn project name: FurrumX.
- [ ] Kiểm tra trademark/domain/package-name collision trước public launch.
- [x] Chọn MIT License và thêm `LICENSE`.
- [x] Dùng baseline copyright `FurrumX contributors`; chưa yêu cầu per-file header.
- [x] Chọn DCO 1.1; không dùng CLA trong baseline.
- [x] Thêm nguyên văn `DCO` và sign-off guidance trong `CONTRIBUTING.md`.
- [ ] Bật automated DCO check khi mở external pull requests.
- [ ] Xác nhận dependency licenses và policy compatibility.
- [ ] Xác định ownership của benchmark datasets/fixtures.
- [ ] Xác định AI-assisted contribution disclosure/provenance policy cuối cùng.

MIT là software license. DCO là certification về quyền gửi contribution, không phải license và không chuyển copyright cho project. Private/internal commits chưa bắt buộc sign-off; external commits sẽ bắt buộc khi repository mở contribution.

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
- [ ] Bật required CI/status checks trên repository host; workflow baseline đã có.
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
- [x] CI baseline cho minimal/local/Flight SQL/S3/Wasm/Python và WSL setup đã được document.
- [ ] Mở rộng CI cho distributed sau khi Ballista compile/runtime spike pass.
- [ ] English translation cho contributor-facing/architecture docs nếu community quốc tế là mục tiêu.

## 5. Release readiness

- [ ] SemVer/API compatibility policy.
- [x] MSRV 1.94 và development toolchain 1.97.1 được ghi trong ADR-0001.
- [x] Platform bootstrap đầu tiên: Linux x86_64/WSL2.
- [x] `Cargo.lock` được tạo; release reproducibility gate vẫn cần artifact verification.
- [ ] Changelog/release notes.
- [ ] Security/advisory process.
- [ ] Performance benchmark manifest/report.
- [ ] Recovery/fault-injection release gates.
- [ ] Container/package registry ownership.

## 6. DCO note

Developer Certificate of Origin là contributor certification, không phải software license. FurrumX dùng nguyên văn DCO 1.1 trong root `DCO`. Khi external contribution được mở, mỗi commit phải có sign-off tạo bởi `git commit -s`; project sẽ bật automated DCO status check trước khi merge.

Tham khảo: [Linux Foundation DCO guidance](https://bestpractices.linuxfoundation.org/ip/contribution-mechanisms-dco.html).
