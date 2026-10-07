import source from "../../../../fixtures/recite/valid/landing-junction.recite?raw";

export function GET() {
  return new Response(source, { headers: { "Content-Type": "text/plain; charset=utf-8" } });
}
