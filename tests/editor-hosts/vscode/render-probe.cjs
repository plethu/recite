// Optional compositor-surface evidence for an isolated measurement host.
const fs = require("node:fs");
const assert = require("node:assert/strict");
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

exports.connect = async function connect(portFile) {
  assert.equal(typeof WebSocket, "function", "render probe requires Node 22+ WebSocket");
  let target;
  const deadline = Date.now() + 30_000;
  while (!target && Date.now() < deadline) {
    if (fs.existsSync(portFile)) {
      const port = Number(fs.readFileSync(portFile, "utf8").split("\n")[0]);
      const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      target = pages.find((page) => page.type === "page" && page.url.includes("workbench"));
    }
    if (!target) await delay(50);
  }
  assert(target, "owned workbench debugging target was not discovered");
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  let serial = 0;
  const pending = new Map();
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    const entry = pending.get(message.id);
    if (!entry) return;
    pending.delete(message.id);
    clearTimeout(entry.timer);
    if (message.error) entry.reject(new Error(JSON.stringify(message.error)));
    else entry.resolve(message.result);
  });
  function call(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++serial;
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(`render command timed out: ${method}`));
      }, 30_000);
      pending.set(id, { resolve, reject, timer });
      socket.send(JSON.stringify({ id, method, params }));
    });
  }
  await call("Page.enable");
  return {
    async capture(missing, broken, destination) {
      const expression = `new Promise((resolve, reject) => {
        const deadline = performance.now() + 10000;
        function check() {
          const editors = [...document.querySelectorAll('.monaco-editor')];
          const editor = editors.find(e => e.querySelector('.view-lines')?.textContent.includes('host_probe'));
          const text = editor?.querySelector('.view-lines')?.textContent ?? '';
          const marks = editor ? [...editor.querySelectorAll('.squiggly-error')].filter(e => e.getBoundingClientRect().width > 0) : [];
          const ready = editor && (${broken} ? text.includes(${
        JSON.stringify(missing)
      }) && marks.length > 0 : text.includes('END') && marks.length === 0);
          if (ready) requestAnimationFrame(() => requestAnimationFrame(() => resolve({ marks: marks.length, text })));
          else if (performance.now() > deadline) reject(new Error('visible diagnostic state timed out: ' + text.slice(-300)));
          else requestAnimationFrame(check);
        }
        check();
      })`;
      const state = await call("Runtime.evaluate", {
        expression,
        awaitPromise: true,
        returnByValue: true,
      });
      assert(!state.exceptionDetails, JSON.stringify(state.exceptionDetails));
      const readyAt = performance.now();
      const image = await call("Page.captureScreenshot", { format: "png", fromSurface: true });
      assert(image.data, "compositor screenshot was empty");
      if (destination) fs.writeFileSync(destination, Buffer.from(image.data, "base64"));
      return { ...state.result.value, readyAt };
    },
    close() {
      socket.close();
    },
  };
};
