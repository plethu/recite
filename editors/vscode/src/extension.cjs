let implementationPromise;

function loadImplementation() {
  return implementationPromise ??= import("./extension.js");
}

module.exports = {
  activate(...args) {
    return loadImplementation().then(({ activate }) => activate(...args));
  },

  async deactivate(...args) {
    if (!implementationPromise) return undefined;
    const { deactivate } = await implementationPromise;
    return deactivate?.(...args);
  }
};
