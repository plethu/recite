import { expect, test } from "@playwright/test";

test("real compiler/runtime load on Run and both branches are playable by keyboard", async ({ page }) => {
  const wasmRequests: string[] = [];
  page.on("request", request => {
    if (request.url().endsWith(".wasm")) wasmRequests.push(request.url());
  });
  await page.goto("/");
  const source = page.getByRole("textbox", { name: "Recite source" });
  await expect(source).toContainText(":: junction default");
  expect(wasmRequests).toEqual([]);
  const transcript = page.getByRole("list", { name: "Dialogue transcript" });
  for (
    const [choice, expected] of [
      ["Restore auxiliary power.", "A contactor closes."],
      ["Unfold the survey plan.", "The paper opens"],
    ]
  ) {
    await page.getByRole("button", { name: "Run scene", exact: true }).focus();
    await page.keyboard.press("Enter");
    await expect(transcript).toContainText("N-2 is stamped");
    for (let step = 0; step < 3; step += 1) {
      const next = page.getByRole("button", { name: "Continue", exact: true });
      await expect(next).toBeEnabled();
      await expect(next).toBeFocused();
      await page.keyboard.press("Enter");
    }
    const reply = page.getByRole("button", { name: choice, exact: true });
    await reply.focus();
    await page.keyboard.press("Enter");
    await expect(transcript).toContainText(expected);
    await page.getByRole("button", { name: "Continue", exact: true }).click();
    await expect(page.getByRole("status")).toContainText("Scene ended");
  }
  expect(wasmRequests.length).toBeGreaterThan(0);
});

test("edited and opened Recite files execute; real diagnostics replace invalid scenes", async ({ page }) => {
  await page.goto("/");
  const source = page.getByRole("textbox", { name: "Recite source" });
  const original = await source.inputValue();
  const transcript = page.getByRole("list", { name: "Dialogue transcript" });
  await source.fill(original.replace("Junction.", "😀 <img src=x onerror=alert(1)>"));
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(transcript).toContainText("😀 <img src=x onerror=alert(1)>");
  await expect(transcript.locator("img")).toHaveCount(0);
  await source.fill(":: start default\r\n> hello@11111111111111111111\r\n  😀\r\n-> missing\r\n");
  await expect(page.getByRole("button", { name: "Continue", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("could not compile");
  await expect(transcript).toContainText("4:");
  await expect(transcript).toContainText("missing");
  await page.getByLabel("Open a .recite file").setInputFiles({
    name: "opened.recite",
    mimeType: "text/plain",
    buffer: Buffer.from(
      ":: start default\n> opened@11111111111111111111\n  From a real file.\n-> END\n",
    ),
  });
  await expect(source).toHaveValue(/From a real file\./);
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(transcript).toContainText("From a real file.");
  await page.getByRole("button", { name: "Reset example" }).click();
  await expect(source).toHaveValue(original);
  await expect(transcript).toBeEmpty();
});

test("cycles and oversized input recover; Stop cancels a pending WASM load", async ({ page }) => {
  await page.goto("/");
  const source = page.getByRole("textbox", { name: "Recite source" });
  const original = await source.inputValue();
  await source.fill(":: loop default\n-> loop\n");
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(page.getByRole("status")).not.toHaveText("Running locally…");
  await expect(page.getByRole("button", { name: "Continue", exact: true })).toHaveCount(0);
  await source.fill("x".repeat(65_537));
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("64 KiB");
  await source.fill(original);
  // Route worker requests so Stop and stale-response fencing are exercised
  // while real initialization is pending, without relying on a slow machine.
  let release: () => void = () => {};
  const blocked = new Promise<void>(resolve => {
    release = resolve;
  });
  await page.route("**/*.wasm", async route => {
    await blocked;
    await route.abort().catch(() => {});
  });
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  const stop = page.getByRole("button", { name: "Stop execution" });
  await expect(stop).toBeEnabled();
  await stop.click();
  await expect(page.getByRole("status")).toContainText("Execution stopped");
  release();
  await page.unroute("**/*.wasm");
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(page.getByRole("list", { name: "Dialogue transcript" })).toContainText(
    "N-2 is stamped",
  );
});

test("effect integers stay exact and memory exhaustion allows a fresh scene", async ({ page }) => {
  await page.goto("/");
  const source = page.getByRole("textbox", { name: "Recite source" });
  const original = await source.inputValue();
  const transcript = page.getByRole("list", { name: "Dialogue transcript" });
  await source.fill(
    ":: start default\n! blocking record(9223372036854775807)\n> done@11111111111111111111\n  Request acknowledged.\n-> END\n",
  );
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(transcript).toContainText("record(9223372036854775807)");
  await page.getByRole("button", { name: "Simulate effect completion" }).click();
  await expect(transcript).toContainText("Request acknowledged.");
  // The runtime stores deferred requests; this loop exceeds the site instance's
  // memory ceiling before the runtime's silent-step limit. No engine effect runs.
  await source.fill(`:: loop default\n! deferred record("${"x".repeat(16_384)}")\n-> loop\n`);
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("preview stopped during execution");
  await expect(page.getByRole("button", { name: "Continue", exact: true })).toHaveCount(0);
  await source.fill(original);
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await expect(transcript).toContainText("N-2 is stamped");
});

test("slow successful initialization has a separate loading deadline", async ({ page, context }) => {
  await page.clock.install();
  let release: () => void = () => {};
  let reached: () => void = () => {};
  const blocked = new Promise<void>(resolve => {
    release = resolve;
  });
  const pending = new Promise<void>(resolve => {
    reached = resolve;
  });
  await context.route("**/*.wasm", async route => {
    reached();
    await blocked;
    await route.continue();
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Run scene", exact: true }).click();
  await pending;
  await page.clock.fastForward(20_000);
  await expect(page.getByRole("status")).toHaveText("Loading compiler…");
  await expect(page.getByRole("button", { name: "Stop execution" })).toBeEnabled();
  release();
  await expect(page.getByRole("list", { name: "Dialogue transcript" })).toContainText(
    "N-2 is stamped",
  );
});
