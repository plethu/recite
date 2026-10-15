const vscode: typeof import("vscode") = require("vscode");

let implementationPromise: Promise<typeof import("./extension.ts")> | undefined;

function loadImplementation() {
  return (implementationPromise ??= import("./extension.js"));
}

module.exports = {
  activate(context: import("vscode").ExtensionContext) {
    return loadImplementation().then(({ activateWithVscode }) =>
      activateWithVscode(vscode, context),
    );
  },

  async deactivate() {
    if (!implementationPromise) return undefined;
    const { deactivateWithVscode } = await implementationPromise;
    return deactivateWithVscode();
  },
};
