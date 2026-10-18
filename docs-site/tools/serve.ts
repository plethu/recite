import { argv, serve, file } from "bun";
import { resolve, sep } from "node:path";
import { redirects } from "./content.ts";
const args = argv.slice(2);
const portIndex = args.indexOf("--port");
const port = portIndex === -1 ? 4173 : Number(args[portIndex + 1]);
const root = resolve(import.meta.dir, "../dist");
const server = serve({
  hostname: "127.0.0.1",
  port,
  async fetch(request) {
    const url = new URL(request.url);
    const redirect = redirects.find(({ from }) => from === url.pathname);
    if (redirect) return Response.redirect(new URL(redirect.to, url), 301);
    let pathname: string;
    try {
      pathname = decodeURIComponent(url.pathname);
    } catch {
      return new Response("Bad request", { status: 400 });
    }
    const path = resolve(root, `.${pathname}`, pathname.endsWith("/") ? "index.html" : "");
    if (!path.startsWith(`${root}${sep}`)) return new Response("Forbidden", { status: 403 });
    const responseFile = file(path);
    if (await responseFile.exists()) return new Response(responseFile);
    return new Response(file(resolve(root, "404.html")), { status: 404 });
  },
});
console.log(`Recite preview: ${server.url}`);
