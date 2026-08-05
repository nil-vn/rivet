# Architecture Decision Record process

## 1. Khi cần ADR

ADR bắt buộc khi thay:

- Architecture invariant hoặc correctness semantics.
- Resume/checkpoint/artifact/manifest contract.
- Event/storage/wire/public API format.
- Arrow/DataFusion/Ballista BOM.
- Runtime plugin boundary hoặc `unsafe` policy.
- External mandatory service/metadata backend.
- Security/trust model.
- Accepted performance regression/complexity trade-off lớn.
- OSS licensing/contribution mechanism/governance.

## 2. Lifecycle

```text
Proposed → Accepted → Superseded
                 └──→ Deprecated
Proposed → Rejected
```

Không chỉnh nội dung decision đã `Accepted` để thay nghĩa lịch sử. Tạo ADR mới và đánh dấu `Supersedes`.

## 3. Location và naming

```text
docs/decisions/0001-short-kebab-title.md
```

Số tăng đơn điệu. `docs/decisions/README.md` liệt kê status.

## 4. Template

```markdown
# ADR-NNNN: Title

- Status: Proposed
- Date: YYYY-MM-DD
- Owners: names/handles
- Deciders: names/roles
- Related: issues/PRs/requirements
- Supersedes: ADR-NNNN or N/A

## Context

Problem, constraints, evidence and current behavior.

## Decision drivers

- Correctness
- Performance
- Operations
- Compatibility
- Security
- Complexity

## Options considered

### Option A

Pros, cons, measurements and risks.

### Option B

Pros, cons, measurements and risks.

## Decision

Chosen option and exact scope.

## Consequences

Positive, negative, migration and follow-up work.

## Verification

Tests, benchmarks, rollout and rollback criteria.
```

## 5. Review requirements

- Correctness-impacting ADR: domain maintainer review.
- Security ADR: security reviewer/private handling nếu nhạy cảm.
- Performance ADR: benchmark/profile evidence.
- Legal/governance ADR: project lead và tư vấn phù hợp; coding agent không tự quyết.

## 6. Implementation

Accepted ADR chưa đồng nghĩa implementation hoàn tất. PR phải link ADR, thêm tests/metrics/migration và cập nhật docs. Nếu evidence khi triển khai bác bỏ assumption, đưa ADR về discussion hoặc tạo superseding ADR.

