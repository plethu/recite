export function SearchDialog() {
  return (
    <dialog id="search-dialog" aria-label="Search documentation" data-pagefind-ignore>
      <form method="dialog">
        <button aria-label="Close search">Close</button>
      </form>
      <p data-search-status role="status"></p>
      <label for="documentation-search">Search</label>
      <div id="search-results"></div>
    </dialog>
  );
}
