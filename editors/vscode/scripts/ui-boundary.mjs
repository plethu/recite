import { parseSource } from "./ui-boundary-ast.mjs";
import { validateAdapter } from "./ui-boundary-adapter.mjs";
import { validateOutside } from "./ui-boundary-calls.mjs";

const ADAPTER_FILE = "user-interface.js";

/**
 * Enforce the small source/UI boundary used by the VS Code package.
 *
 * The adapter and caller policy are structural checks. They deliberately
 * avoid general source-level dataflow and alias resolution.
 */
export function assertUiBoundary(sourceFiles, ownedIds, projectedMessages, adapterFile = ADAPTER_FILE) {
  const entries = new Map(sourceFiles);
  const adapterSource = entries.get(adapterFile);
  assert(typeof adapterSource === "string", `UI adapter ${adapterFile} is required`);
  const expected = new Set(ownedIds);
  for (const id of expected) {
    assert(Object.hasOwn(projectedMessages, id), `UI message projection is missing ${id}`);
  }

  const contracts = validateAdapter(parseSource(adapterSource, adapterFile), adapterFile, expected);
  for (const [file, source] of entries) {
    if (file === adapterFile || file === "messages.js") continue;
    validateOutside(parseSource(source, file), file, contracts);
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
