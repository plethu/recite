import { readFile, readdir } from "node:fs/promises";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";

const repoRoot = process.argv[2] ?? join(dirname(fileURLToPath(import.meta.url)), "..");
const schema = JSON.parse(await readFile(join(repoRoot, "schemas/recite-schema-manifest-v1.schema.json"), "utf8"));
const ajv = new Ajv2020({ allErrors: true, strict: true, allowUnionTypes: true });
const validate = ajv.compile(schema);

async function documents(directory) {
  return (await readdir(directory)).filter((name) => name.endsWith(".json")).sort();
}

for (const name of await documents(join(repoRoot, "fixtures/schema/valid"))) {
  const document = JSON.parse(await readFile(join(repoRoot, "fixtures/schema/valid", name), "utf8"));
  if (!validate(document)) throw new Error(`valid fixture rejected: ${name}\n${ajv.errorsText(validate.errors)}`);
}

const shapeInvalid = [
  "array_type_reference.json",
  "contextual_missing_context.json",
  "invalid_export_version.json",
  "legacy_string_origin.json",
  "malformed_shape.json",
  "malformed_provenance.json",
  "unnamespaced_provenance_field.json",
  "unknown_top_level_producer_field.json",
  "unsupported_version.json",
];
for (const name of shapeInvalid) {
  const document = JSON.parse(await readFile(join(repoRoot, "fixtures/schema/invalid", name), "utf8"));
  if (validate(document)) throw new Error(`shape-invalid fixture accepted: ${name}`);
}
console.log(`JSON Schema validated ${ (await documents(join(repoRoot, "fixtures/schema/valid"))).length } valid and ${shapeInvalid.length} shape-invalid fixtures`);
