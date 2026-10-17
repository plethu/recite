// HTML is produced from trusted repository Markdown by tools/markdown.ts.
export function RenderedMarkdown({ html }: { html: string }) {
  return <div dangerouslySetInnerHTML={{ __html: html }} />;
}
