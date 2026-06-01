# Codeberg Issue and PR Examples

Use these examples when creating or updating Recite Codeberg issues and pull requests. Keep remote mutations sequential and route them through `scripts/tea-rate-limit.sh`.

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
- `scripts/check-project-gates.sh`

## Spec References
- `docs/recite-production-spec.md` §5.2

## Suggested Branch
`issue-N-parser-block-headers`
EOF

.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea issues create \
    --title "Parser: parse block headers" \
    --labels "area/parser,kind/implementation,size/s,status/ready" \
    --description "$(cat "$tmp_body")"
```

## Move One Issue To Review

```bash
.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea issues edit \
    --remove-labels "status/in-progress" \
    --add-labels "status/review" \
    17
```

## Open A Pull Request

```bash
tmp_body="$(mktemp)"
cat > "$tmp_body" <<'EOF'
Closes #17

## Summary
Adds parser support for block headers with source spans.

## Tests
- `scripts/check-project-gates.sh`
EOF

.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea pulls create \
    --head issue-17-parser-block-headers \
    --base main \
    --title "Parser: parse block headers" \
    --description "$(cat "$tmp_body")"
```
