export function PageTitle({ children }: { children: string }) {
  return (
    <h1 id="_top" tabindex={-1} data-pagefind-meta="title">
      {children}
    </h1>
  );
}
