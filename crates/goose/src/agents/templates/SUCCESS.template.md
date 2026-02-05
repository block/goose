# Success Criteria

This file defines HARD PASS/FAIL checks. The runbook is NOT complete until ALL criteria pass.

## Automated Checks

These can be verified automatically:

- **Build succeeds**
  Check: `cargo build --release`
  Expect: Exit code 0, no errors

- **All tests pass**
  Check: `cargo test --workspace`
  Expect: "test result: ok"

- **No clippy warnings**
  Check: `./scripts/clippy-lint.sh`
  Expect: Exit code 0

- **Formatting clean**
  Check: `cargo fmt -- --check`
  Expect: No files need formatting

- **[Add project-specific automated checks]**
  Check: `[command]`
  Expect: `[expected output]`

## Manual Verification

These require human check:

- [ ] Documentation updated and accurate
- [ ] Breaking changes noted in CHANGELOG
- [ ] Performance benchmarks acceptable
- [ ] UI changes reviewed visually (if applicable)
- [ ] Security implications considered

## Integration Checks

- **Dependencies secure**
  Check: `cargo audit`
  Expect: No vulnerabilities

- **Upstream sync clean**
  Check: `git status`
  Expect: "working tree clean"

## Definition of Done

ALL of the following must be true:
- [ ] All automated checks pass
- [ ] All manual verification items checked
- [ ] Integration checks pass
- [ ] No TODO or FIXME comments in changed code
- [ ] No placeholder/stub implementations
- [ ] Documentation reflects actual implementation

---

**CRITICAL:** Do not mark task as complete unless EVERY item above is satisfied.
