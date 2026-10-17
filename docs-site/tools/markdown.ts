import MarkdownIt, { type Token } from "markdown-it";
import anchor from "markdown-it-anchor";
import { slug } from "github-slugger";
import { bundledLanguages, createCssVariablesTheme, createHighlighter } from "shiki";
import reciteGrammar from "../../editors/vscode/syntaxes/recite.tmLanguage.json";

export interface Heading {
  id: string;
  text: string;
  level: number;
}

export function fencedLanguages(sources: string[]) {
  const md = new MarkdownIt({ html: true });
  const languages = new Set<string>();
  function visit(tokens: Token[]) {
    for (const token of tokens) {
      if (token.type === "fence") {
        const language = md.utils.unescapeAll(token.info).trim().split(/\s+/, 1)[0];
        if (language && language !== "text" && language !== "recite") languages.add(language);
      }
      if (token.children) visit(token.children);
    }
  }
  for (const source of sources) visit(md.parse(source, {}));
  return [...languages];
}

export async function createMarkdown(languages: string[]) {
  const highlighter = await createHighlighter({
    themes: [createCssVariablesTheme({ variablePrefix: "--recite-code-" })],
    langs: [
      { ...reciteGrammar, name: "recite" },
      ...languages.map((name) => {
        if (!(name in bundledLanguages)) throw new Error(`Unknown fenced code language: ${name}`);
        return bundledLanguages[name as keyof typeof bundledLanguages];
      }),
    ],
  });
  const md = new MarkdownIt({
    // Content is maintained in this repository; HTML carries language/direction metadata.
    html: true,
    highlight(code, language) {
      return highlighter
        .codeToHtml(code, { lang: language || "text", theme: "css-variables" })
        .replace("<pre ", '<pre tabindex="0" ');
    },
  }).use(anchor, { slugify: slug, tabIndex: false });
  const table = md.renderer.rules.table_open;
  md.renderer.rules.table_open = (tokens, index, options, env, renderer) => {
    tokens[index]!.attrSet("tabindex", "0");
    return table
      ? table(tokens, index, options, env, renderer)
      : renderer.renderToken(tokens, index, options);
  };
  return {
    render(source: string) {
      const tokens = md.parse(source, {});
      const headings: Heading[] = [];
      tokens.forEach((token, index) => {
        if (token.type === "heading_open")
          headings.push({
            id: String(token.attrGet("id")),
            text: tokens[index + 1]!.content,
            level: Number(token.tag.slice(1)),
          });
      });
      return { html: md.renderer.render(tokens, md.options, {}), headings };
    },
    dispose: () => highlighter.dispose(),
  };
}
