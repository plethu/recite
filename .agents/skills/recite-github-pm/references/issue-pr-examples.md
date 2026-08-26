# GitHub Issue And PR Examples

Use these examples when creating or updating Recite GitHub issues and pull
requests. Keep remote mutations sequential and pass the explicit repository to
every `gh` command.

## Create One Issue

```bash
tmp_body="$(mktemp)"
cat > "$tmp_body" <<'EOF'
## Goal
Parse block headers with stable source spans.

## Scope
Parser and AST behavior for block headers only.

## Known Decisions
Runtime traversal is out of scope. Diagnostics should carry source spans.

## Open Questions
None known.

## Acceptance Criteria
- Parses named blocks.
- Rejects malformed block headers with a span.
- Adds focused parser tests.

## Out of Scope
Diverts, choices, runtime traversal, and LSP diagnostics.

## Test/Check Commands
- `mise run verify`

## Spec References
- `docs/recite-production-spec.md` §5.2

## Suggested Branch
`feat/parser-block-headers`
EOF

gh issue create --repo plethu/recite \
  --title "Parser: parse block headers" \
  --label "area/parser,kind/implementation,size/s,status/ready" \
  --body-file "$tmp_body"
```

## Move One Issue To Review

```bash
gh issue edit 17 --repo plethu/recite \
  --remove-label "status/in-progress" \
  --add-label "status/review"
```

## Open A Pull Request

```bash
tmp_body="$(mktemp)"
cat > "$tmp_body" <<'EOF'
Closes #17

## Summary
Adds parser support for block headers with source spans.

## Tests
- `mise run verify`
EOF

gh pr create --repo plethu/recite \
  --head feat/parser-block-headers \
  --base main \
  --title "Parser: parse block headers" \
  --body-file "$tmp_body"
```
