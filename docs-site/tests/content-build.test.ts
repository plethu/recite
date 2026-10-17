import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "bun:test";
import { contentUrl, loadContent } from "../tools/content.ts";
import { createMarkdown } from "../tools/markdown.ts";

test("published document URLs retain section indexes and reject unsafe filenames", () => {
  assert.equal(contentUrl("reference/index.md"), "/reference/");
  assert.equal(contentUrl("getting-started/first-scene.md"), "/getting-started/first-scene/");
  assert.equal(contentUrl("game.md", "/showcase"), "/showcase/game/");
  for (const name of ["../outside.md", "/outside.md", "a/../../outside.md", "not a slug.md"])
    assert.throws(() => contentUrl(name), /Unsafe content path/);
});

test("Markdown preserves punctuation fragments, script metadata and keyboard scrolling", async () => {
  const markdown = await createMarkdown(["bash"]);
  try {
    const rendered = markdown.render(
      '## `recite --help`\n\n## `recite --help`\n\n<div lang="ja">送信機</div>\n\n```bash\nrecite --help\n```\n\n| A | B |\n| - | - |\n| x | y |\n',
    );
    assert.deepEqual(
      rendered.headings.map(({ id }) => id),
      ["recite---help", "recite---help-1"],
    );
    assert.match(rendered.html, /<div lang="ja">送信機<\/div>/);
    assert.match(rendered.html, /<pre tabindex="0"/);
    assert.match(rendered.html, /<table tabindex="0"/);
    assert.match(rendered.html, /--recite-code-/);
  } finally {
    markdown.dispose();
  }
});

test("the build validates frontmatter context and only publishes real nondraft projects", async () => {
  const root = await mkdtemp(join(tmpdir(), "recite-site-content-"));
  const docs = join(root, "src/content/docs");
  const showcase = join(root, "src/content/showcase");
  await Promise.all([mkdir(docs, { recursive: true }), mkdir(showcase, { recursive: true })]);
  try {
    await writeFile(
      join(docs, "index.md"),
      "---\ntitle: Manual\ndescription: Read me\n---\n## Start\n",
    );
    await writeFile(
      join(showcase, "game.md"),
      "---\ntitle: Game\nsummary: A real project\nprojectUrl: https://example.com/\n---\nProject body.\n",
    );
    await writeFile(
      join(showcase, "draft.md"),
      "---\ntitle: Draft\nsummary: Not ready\nprojectUrl: https://example.com/\ndraft: true\n---\n",
    );
    const content = await loadContent(root);
    assert.equal(content.documents[0]?.title, "Manual");
    assert.deepEqual(
      content.projects.map(({ url }) => url),
      ["/showcase/game/"],
    );
    await writeFile(
      join(showcase, "fixture.md"),
      "---\ntitle: Fixture\nsummary: Not a project\nprojectUrl: https://example.com/\nfixture: true\n---\n",
    );
    await assert.rejects(loadContent(root), /showcase\/fixture.md: fixture:/);
    await rm(join(showcase, "fixture.md"));
    await writeFile(join(docs, "index.md"), "---\ntitle: Manual\ndescription: 42\n---\n");
    await assert.rejects(loadContent(root), /index.md: description:/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
