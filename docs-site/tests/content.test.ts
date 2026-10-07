import assert from "node:assert/strict";
import { test } from "node:test";
import { showcaseSchema } from "../src/showcase-schema.ts";

const project = {
  title: "Test project",
  summary: "Test metadata",
  projectUrl: "https://example.com/",
};

test("showcase defaults to public and rejects investigation-only metadata", () => {
  assert.equal(showcaseSchema.parse(project).draft, false);
  assert.equal(showcaseSchema.parse({ ...project, draft: true }).draft, true);
  assert.equal(showcaseSchema.safeParse({ ...project, fixture: true }).success, false);
});

test("showcase metadata requires a real HTTPS project URL", () => {
  assert.equal(showcaseSchema.safeParse(project).success, true);
  for (const projectUrl of ["not-a-url", "http://example.com/", "javascript:alert(1)"]) {
    const result = showcaseSchema.safeParse({ ...project, projectUrl });
    assert.equal(result.success, false);
    if (!result.success) assert.deepEqual(result.error.issues[0]?.path, ["projectUrl"]);
  }
});

test("showcase images require paired nonempty alternative text", () => {
  for (
    const image of [{ image: "/example.svg" }, { imageAlt: "An example" }, {
      image: "/example.svg",
      imageAlt: "",
    }]
  ) {
    assert.equal(showcaseSchema.safeParse({ ...project, ...image }).success, false);
  }
  assert.equal(
    showcaseSchema.safeParse({ ...project, image: "/example.svg", imageAlt: "An example" }).success,
    true,
  );
});
