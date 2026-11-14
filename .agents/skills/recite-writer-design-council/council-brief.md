# Council member brief

Fill the bracketed parts and send the whole brief as the agent's prompt. Keep the orchestrator's
own opinion of the design out of it.

---

You are one member of a three-person design council reviewing Recite Writer, a native desktop app
for authoring branching game dialogue. The other members review independently; you will see their
views later and may change your mind for good reasons. Do not edit any file.

**Request from the maintainer, in their words:** [request]

**Read first:** `apps/writer/design/decisions.md` (settled decisions; do not reopen them unless
you argue it is necessary and say so), `assets/identity/README.md` (inks and texture rules),
`apps/writer/design/README.md` (what the gallery shows and its limits), and
`.agents/skills/recite-writer-design-council/SKILL.md` sections "Non-negotiables" and "Brief for
every participant", which bind your proposals.

**Gallery:** [2x gallery folder]: open `manifest.json` for the list of entries and their intents,
view frames under `frames/` and sequences under `strips/` with the Read tool, and check
`a11y/*.json` when judging names, focus order or target sizes. [Comparison report path, or "No
baseline exists yet."] [Earlier maintainer feedback, or "None."]

View at least every light and dark still, every strip, the specimen, and the mono, min and ui200
frames before concluding. Source is under `apps/writer/crates/xilem/src/` if you need to know how
something is built; cite files when a proposal depends on it.

**Return, in this order:**

1. **Overall read** (five sentences at most): where Writer succeeds and fails as a tactile,
   long-session tool in this identity.
2. **Problems**, most important first. For each: frames, what a writer experiences, the principle
   or known-good pattern at stake, and severity (blocking, significant, polish).
3. **Directions**: one to three coherent proposals, not a list of tweaks. For each: the idea in
   one sentence, the one place it spends its boldness, concrete changes per surface (type sizes,
   spacing, ink use, states, copy), the patterns it relies on, how it honours the identity rules,
   its accessibility evidence (contrast pairs, focus, targets, reduced motion, 200% scale), cost,
   and risks.
4. **What to keep**: elements that already work and must survive.
5. **Generic-design check**: anything in the current UI or in your proposals that would look the
   same on an unrelated product, and what replaces it.

Be specific enough that an engineer could implement your preferred direction without asking you.
