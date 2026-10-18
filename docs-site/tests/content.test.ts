import assert from "node:assert/strict";
import { test } from "bun:test";
import { parse, safeParse, getDotPath } from "valibot";
import { showcaseSchema } from "../src/showcase-schema.ts";

const project = {
  title: "Test project",
  summary: "Test metadata",
  projectUrl: "https://example.com/",
};

test("showcase defaults to public and rejects investigation-only metadata", () => {
  assert.equal(parse(showcaseSchema, project).draft, false);
  assert.equal(parse(showcaseSchema, { ...project, draft: true }).draft, true);
  assert.equal(safeParse(showcaseSchema, { ...project, fixture: true }).success, false);
});

test("showcase metadata requires a real HTTPS project URL", () => {
  assert.equal(safeParse(showcaseSchema, project).success, true);
  for (const projectUrl of ["not-a-url", "http://example.com/", "javascript:alert(1)"]) {
    const result = safeParse(showcaseSchema, { ...project, projectUrl });
    assert.equal(result.success, false);
    if (!result.success) assert.deepEqual(getDotPath(result.issues[0]!), "projectUrl");
  }
});

test("showcase images require paired nonempty alternative text", () => {
  for (const image of [
    { image: "/example.svg" },
    { imageAlt: "An example" },
    {
      image: "/example.svg",
      imageAlt: "",
    },
  ]) {
    assert.equal(safeParse(showcaseSchema, { ...project, ...image }).success, false);
  }
  assert.equal(
    safeParse(showcaseSchema, { ...project, image: "/example.svg", imageAlt: "An example" })
      .success,
    true,
  );
});
