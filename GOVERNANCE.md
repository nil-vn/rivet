# Governance

## 1. Mục tiêu

Governance bảo vệ ba giá trị: correctness có thể kiểm toán, hiệu năng có bằng chứng và cộng đồng mở nhưng có trách nhiệm.

## 2. Vai trò

### Contributor

- Mở issue, docs, test, benchmark hoặc code contribution.
- Chịu trách nhiệm nguồn gốc và chất lượng nội dung gửi lên.

### Reviewer

- Có kinh nghiệm ở một domain.
- Review correctness, maintainability, performance evidence và test coverage.
- Không merge chỉ vì patch “chạy được”.

### Maintainer

- Triage, approve/merge, release và quản lý roadmap.
- Bảo vệ architecture invariants và compatibility.
- Quản lý security/private reports.
- Công khai conflict of interest khi liên quan.

### Project lead

- Quyết định cuối cùng khi maintainers không đạt đồng thuận.
- Chốt legal/trademark/release governance.
- Không được bỏ qua security/correctness invariant mà không có ADR và documented risk acceptance.

Danh sách cá nhân/organization sẽ được thêm vào `MAINTAINERS.md` hoặc CODEOWNERS sau khi repository owner xác định public identities.

## 3. Quyết định

- Changes nhỏ: lazy consensus qua PR review.
- Architecture/breaking changes: ADR và ít nhất required maintainer approvals theo branch policy.
- Security fixes: private coordination, embargo khi cần.
- Performance trade-offs: benchmark evidence và explicit complexity review.
- Legal/license/DCO/CLA: project lead quyết định sau tư vấn phù hợp; không do code PR ngẫu nhiên quyết định.

## 4. Quyền block merge

Một concern có evidence về các mục sau có thể block merge:

- Data loss/duplicate/corruption.
- Resume/commit/fencing violation.
- Security vulnerability.
- Unbounded memory/concurrency.
- Incompatible Arrow/DataFusion/Ballista types.
- Performance regression đáng kể trên declared critical workload.
- Thiếu license/provenance của dependency/code/data.

Blocker phải nêu scenario/evidence và điều kiện để resolve; không dùng veto mơ hồ.

## 5. AI-assisted contributions

- Coding agent không phải maintainer/reviewer pháp lý.
- Human PR owner chịu trách nhiệm provenance, correctness, tests và disclosures.
- Agent output được review như code chưa tin cậy.
- Không đưa secrets/private code vào external model/tool trái policy.
- Agent không tự merge/release/publish.

## 6. Release governance

- Reproducible locked dependency graph.
- Security/dependency/license checks.
- Correctness, recovery và performance release gates.
- Changelog/migration notes.
- Benchmark claims kèm manifest.
- Signed artifacts/SBOM khi release pipeline được thiết lập.

## 7. Thay đổi governance

Governance change cần public proposal, thời gian phản hồi hợp lý và project lead approval. Không retroactively thay contribution licensing/provenance terms cho contribution cũ.

