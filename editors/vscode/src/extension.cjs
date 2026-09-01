const vscode = require("vscode");

let implementationPromise;

function loadImplementation() {
  return implementationPromise ??= import("./extension.js");
}

module.exports = {
  activate(...args) {
    return loadImplementation().then(({ activateWithVscode }) =>
      activateWithVscode(vscode, ...args));
  },

  async deactivate(...args) {
    if (!implementationPromise) return undefined;
    const { deactivateWithVscode } = await implementationPromise;
    return deactivateWithVscode(...args);
  }
};
