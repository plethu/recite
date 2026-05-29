---
name: recite-agent-instructions
description: Use when authoring or editing agent-facing instructions in this repo — CLAUDE.md, AGENTS.md, and .agents/skills/*/SKILL.md. Applies research-backed principles for minimal, procedural, non-redundant guidance.
---

# Recite Agent Instructions Authoring

## Why

Agent instruction files are not free. Empirical evaluation shows that adding
repository context files **tends to reduce** coding-agent success and increase
cost by >20%, and that overlong or comprehensive guidance is net-negative. The
default assumption "more guidance helps" is wrong. Edit these files like you
would edit production code: every line must earn its place by changing agent
behavior for the better.

Use this skill for any change to `CLAUDE.md`, `AGENTS.md`, or
`.agents/skills/*/SKILL.md`.

## Evidence Base

Two findings drive the rules below:

- **Repository context files (`AGENTS.md`/`CLAUDE.md`)** — LLM-generated context
  files are net-negative on task success; developer-written ones give only a
  marginal gain and still raise cost (more steps, more reasoning tokens).
  Unnecessary requirements make tasks *harder*. Codebase overviews and listings
  of easily-discoverable structure are redundant and effectively ignored. But
  instructions *are* followed: naming a specific tool or command sharply
  increases its correct use. (arXiv:2602.11988)
- **Skills** — Curated, human-authored skills help substantially; self-generated
  ones do not. **2–3 focused modules** is optimal; 4+ shows diminishing returns.
  **Detailed or compact** guidance beats **comprehensive** documentation, which
  is net-negative. Skills win on concrete *procedural* steps, constraints, and a
  worked example — and on workflows underrepresented in model priors. They
  *hurt* on tasks the model already handles well. (arXiv:2602.12670)

## When To Add Or Edit

Before writing, decide whether the instruction should exist at all:

| Situation | Action |
| --- | --- |
| Fact is discoverable by reading the repo (structure, file names, obvious build) | Do not add it. Agents re-derive this regardless. |
| Behavior the model already gets right by default | Do not add it. Redundant guidance is net-negative. |
| Non-obvious required tool, command, convention, or invariant | Add it — tool/command instructions are reliably followed. |
| Procedure with a brittle format, ordering, or constraint that is easy to get wrong | Add it as concise procedural steps. |
| General background or conceptual overview | Omit. Prefer a spec pointer (`docs/recite-production-spec.md §N`). |

## Authoring Principles

- **Minimal requirements only.** State the smallest set of non-obvious
  constraints needed. Cut anything the agent would do correctly without it.
- **Procedural, not declarative.** Describe *how* to do a class of tasks (steps,
  ordering, sanity checks), not background facts or what-to-output answers.
- **Concrete and verifier-facing.** Prefer specific commands, named tools, exact
  constraints, and one short worked example over abstract advice.
- **Moderate length wins.** Detailed-but-compact beats exhaustive. If a section
  reads like comprehensive documentation, it is probably hurting.
- **Steer tooling explicitly.** Naming the right command (e.g. `cargo clippy
  --all-targets --all-features -- -D warnings`, a specific test path) is one of
  the highest-value, reliably-followed instructions.
- **Match harness/format constraints.** When output must follow a strict format
  (JSON, structured diagnostics, stable IDs), restate the constraint at the
  point of use rather than once at the top.
- **Class-general, not instance-specific.** Guidance must apply to a class of
  tasks. Never embed solutions to one task, magic numbers, or specific answers.

## Anti-Patterns To Remove

- Codebase overviews, directory tours, or file listings of discoverable structure.
- Restating defaults the model already follows.
- Long conceptual essays; replace with a spec section pointer.
- A 4th+ overlapping skill or section covering the same concern — consolidate
  toward 2–3 focused modules.
- Copied large spans of `docs/recite-production-spec.md` (the project forbids
  this; link the section instead).
- Self-generated boilerplate added "for completeness."

## Editing SKILL.md Files

- Keep each skill scoped to one responsibility named in its `description`.
- Frontmatter `description` is the routing signal: it must state *when* to use
  the skill, in concrete trigger terms, so it is selected correctly.
- Open with a short `## Why`, then procedural sections; end with a `## Handoff`
  or checklist when the skill gates work.
- Prefer tables and short imperative bullets over prose.
- If a skill grows toward comprehensive coverage, split or trim — moderate length
  outperforms exhaustive.

## Editing CLAUDE.md

- It is the always-loaded context budget; treat additions as expensive.
- Keep it to workflow, product invariants, spec routing, and skill pointers.
- Push procedural detail down into the relevant `SKILL.md`; CLAUDE.md should
  route to skills, not duplicate them.
- Verify any spec section numbers against the committed spec before editing the
  routing table.

## Handoff

Before handoff, state:

- What instruction was added/changed and the specific behavior it should change.
- Why it is not already discoverable or default (the bar for inclusion).
- What you removed or declined to add to keep guidance minimal.
- For SKILL.md changes: confirm the `description` routes correctly and the skill
  stays within one responsibility.
