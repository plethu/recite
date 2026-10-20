import { basename, resolve } from "node:path";
import { build, file, write, CryptoHasher } from "bun";

export async function buildAssets(root: string) {
  const outdir = resolve(root, "dist/assets");
  async function bundle(entry: string) {
    const result = await build({
      entrypoints: [resolve(root, entry)],
      outdir,
      naming: "[name]-[hash].[ext]",
      target: "browser",
      minify: true,
      external: ["/fonts/*"],
    });
    if (!result.success) throw new AggregateError(result.logs, `Could not bundle ${entry}`);
    const output = result.outputs.find(
      (asset) => asset.kind === "entry-point" || asset.loader === "css",
    );
    if (!output) throw new Error(`No browser asset emitted for ${entry}`);
    return `/assets/${basename(output.path)}`;
  }
  async function copy(name: string) {
    const contents = await file(resolve(root, "src/client/generated", name)).arrayBuffer();
    const hash = new CryptoHasher("sha256").update(contents).digest("hex").slice(0, 12);
    const output = name.replace(/(\.[^.]+)$/, `-${hash}$1`);
    await write(resolve(outdir, output), contents);
    return `/assets/${output}`;
  }
  const [site, controller, worker, css, module, binary] = await Promise.all([
    bundle("src/client/site.ts"),
    bundle("src/client/controller.ts"),
    bundle("src/client/worker.ts"),
    bundle("src/styles/site.css"),
    copy("recite_playground.js"),
    copy("recite_playground_bg.wasm"),
  ]);
  return { site, controller, worker, css, module, binary };
}
