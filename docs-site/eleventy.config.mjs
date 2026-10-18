import { rm } from "node:fs/promises";
import { resolve } from "node:path";
import { buildSite } from "./tools/site.ts";
import { writeSiteOutput } from "./tools/output.ts";

export default function (config) {
  let site;
  config.addExtension("11ty.tsx", {
    key: "11ty.js",
    compile() {
      return async function (data) {
        return "<!doctype html>" + (await this.defaultRenderer(data));
      };
    },
  });
  config.addTemplateFormats("11ty.tsx");
  // Bun watches imported TS/TSX; Eleventy’s JS dependency parser cannot parse them.
  config.setWatchJavaScriptDependencies(false);
  config.addWatchTarget("./src/**/*");
  config.addWatchTarget("./tools/**/*");
  // Absolute external paths avoid remapping the content watcher’s working directory.
  config.addWatchTarget(resolve("../assets/identity/**/*"));
  config.addWatchTarget(resolve("../fixtures/recite/valid/landing-junction.recite"));
  config.addPassthroughCopy({ public: "/" });
  config.addGlobalData("site", () => site);
  config.on("eleventy.before", async () => {
    await rm("dist", { recursive: true, force: true });
    site = await buildSite(process.cwd());
  });
  config.on("eleventy.after", ({ results }) => {
    const urls = results.map((page) => page.url).filter((url) => url && url !== "/404.html");
    return writeSiteOutput(site, resolve("dist"), urls);
  });
  return { dir: { input: "src/pages", output: "dist" }, templateFormats: ["11ty.tsx"] };
}
