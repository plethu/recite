// Deliberately small fixture set: this study demonstrates interaction, not locale validation.
const languages = [
 ['English (United Kingdom)', '', 'en-GB'], ['English (United States)', '', 'en-US'],
 ['French', 'Français', 'fr'], ['French (Belgium)', 'Français · Belgique', 'fr-BE'],
 ['French (Canada)', 'Français · Canada', 'fr-CA'], ['French (France)', 'Français · France', 'fr-FR'],
 ['French (Luxembourg)', 'Français · Luxembourg', 'fr-LU'], ['French (Switzerland)', 'Français · Suisse', 'fr-CH'],
 ['German', 'Deutsch', 'de'], ['Japanese', '日本語', 'ja'], ['Portuguese (Brazil)', 'Português · Brasil', 'pt-BR'],
 ['Welsh', 'Cymraeg', 'cy'], ['Welsh (Cofi)', 'Cymraeg · Cofi', 'cy-x-cofi']
];
const $ = id => document.getElementById(id);
const input = $('language'), popover = $('popover'), results = $('results');
let selected = null, matches = [], active = -1;
function close() { popover.hidden = true; input.setAttribute('aria-expanded', 'false'); input.removeAttribute('aria-activedescendant'); }
function destination() {
 $('destination').replaceChildren();
 if (selected) {
  const code = document.createElement('code'); code.textContent = `locale/${selected[2]}.po`;
  $('destination').append('Creates ', code);
 } else $('destination').textContent = 'Choose a language to see its catalogue location.';
 $('create').disabled = !selected;
}
function highlight(index) {
 active = index;
 [...results.children].forEach((node, i) => node.setAttribute('aria-selected', String(i === index)));
 if (index >= 0) { input.setAttribute('aria-activedescendant', `option-${index}`); results.children[index].scrollIntoView({block:'nearest'}); }
 else input.removeAttribute('aria-activedescendant');
}
function choose(item) { selected = item; input.value = item[0]; close(); destination(); input.focus(); }
function search() {
 popover.hidden = false; input.setAttribute('aria-expanded', 'true');
 const query = input.value.trim().toLocaleLowerCase();
 matches = query ? languages.filter(item => item.some(part => part.toLocaleLowerCase().includes(query))) : [];
 results.replaceChildren();
 matches.forEach((item, i) => {
  const row = document.createElement('div'); row.className = 'option'; row.id = `option-${i}`; row.setAttribute('role', 'option');
  const name = document.createElement('span'); name.textContent = item[0];
  if (item[1]) { const native = document.createElement('small'); native.textContent = item[1]; name.append(native); }
  const code = document.createElement('code'); code.textContent = item[2]; row.append(name, code);
  row.addEventListener('mousedown', event => event.preventDefault()); row.addEventListener('click', () => choose(item)); results.append(row);
 });
 $('hint').textContent = !query ? 'Search by language, native name or locale code. For example, Français or fr-CA.' : matches.length ? `${matches.length} matches · ↑ ↓ to move · Enter to choose` : 'No matching languages. Try a language name or locale code.';
 highlight(-1);
}
input.addEventListener('click', () => { if (popover.hidden) { input.select(); search(); } });
input.addEventListener('input', () => { selected = null; destination(); search(); });
input.addEventListener('keydown', event => {
 if (event.key === 'Escape') { close(); return; }
 if (event.key === 'Tab') { close(); return; }
 if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
  event.preventDefault(); if (popover.hidden) search();
  if (matches.length) highlight((active + (event.key === 'ArrowDown' ? 1 : -1) + matches.length) % matches.length);
 }
 if (event.key === 'Enter' && !popover.hidden && matches.length) { event.preventDefault(); choose(matches[Math.max(0,active)]); }
});
$('toggle').onclick = () => { if (popover.hidden) { input.focus(); input.select(); search(); } else close(); };
document.addEventListener('pointerdown', event => { if (!event.target.closest('.picker')) close(); });
const notes = {
 closed:['Closed','One field, one decision. The catalogue name follows the selected locale.'],
 searching:['Searching','The result panel floats over the form. The dialog, field and actions stay in place. Scroll for more matches.'],
 selected:['Selected','The locale determines the filename. The next action is ready, without another naming decision.']
};
function show(state) {
 close(); selected = null; input.value = ''; $('feedback').textContent = '';
 if (state === 'selected') { selected = languages.find(item => item[2] === 'fr-CA'); input.value = selected[0]; }
 destination();
 document.querySelectorAll('[data-state]').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.state === state)));
 $('state-title').textContent = notes[state][0]; $('state-note').textContent = notes[state][1];
 if (state === 'searching') { input.value = 'French'; input.focus(); search(); highlight(2); }
}
document.querySelectorAll('[data-state]').forEach(button => button.onclick = () => show(button.dataset.state));
$('cancel').onclick = () => { show('closed'); $('feedback').textContent = 'Cancelled · This study keeps the dialog visible for comparison.'; };
$('create').onclick = () => { $('feedback').textContent = `Would create locale/${selected[2]}.po and return to the passage. No file was written.`; };
$('theme').onclick = () => { document.body.classList.toggle('light'); $('theme').textContent = document.body.classList.contains('light') ? 'Dark theme' : 'Light theme'; };
show('closed');
