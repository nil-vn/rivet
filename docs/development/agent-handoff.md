# Agent collaboration và handoff

## 1. Mục tiêu

Cho phép nhiều coding agents làm việc mà không ghi đè nhau, không mất context và không tạo claims không kiểm chứng.

## 2. Task assignment

Mỗi agent task cần:

- Objective cụ thể.
- File/module ownership dự kiến.
- Inputs/requirements cần đọc.
- Output/acceptance criteria.
- Tests/benchmarks mong đợi.
- Explicit non-goals.

Không phân hai agent sửa cùng file/hot path đồng thời trừ khi có coordinator rõ.

## 3. Trước khi sửa

```text
read AGENTS.md
read relevant architecture/domain docs
inspect git status and existing diff
confirm no overlapping ownership
state assumptions and risk
```

Agent không được coi uncommitted files là disposable.

## 4. Trong khi làm

- Giữ scope focused.
- Thông báo coordinator khi thay đổi interface/file ownership.
- Không sửa unrelated formatting.
- Không tự thay requirement để code dễ hơn.
- Ghi evidence cho test/benchmark claims.
- Nếu phát hiện blocker architecture/legal/security, dừng phần bị block và báo rõ.

## 5. Handoff template

```markdown
## Outcome

What is now true for the user/project.

## Files changed

- path: purpose

## Requirements and decisions

- Requirement IDs
- ADRs affected/created

## Verification

- Command: result
- Benchmark: environment and manifest

## Not verified

- Command/test not run and reason

## Remaining risks

- Correctness
- Performance
- Security
- Compatibility

## Follow-ups

- Bounded next actions
```

## 6. Review of agent output

Human/parent agent phải:

- Đọc actual diff, không chỉ handoff summary.
- Verify tests/benchmark outputs.
- Kiểm tra provenance/license.
- Tìm unsupported claims.
- Kiểm tra shared-workspace conflicts.
- Chịu trách nhiệm trước merge.

## 7. Agent-specific prohibitions

- Không tự chọn LICENSE/DCO/CLA.
- Không tự publish package/release/container.
- Không đưa secrets/private data vào tool prompts/logs.
- Không đánh dấu test pass nếu chưa chạy.
- Không dùng generated benchmark numbers.
- Không bypass review bằng mass-generated code/docs.

