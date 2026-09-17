/* Disposable interaction study. All drafts and simulated file changes are in memory. */
'use strict';
const $ = id => document.getElementById(id);
const entries = [
  {beat:'Station History', speaker:'Mara', kind:'Dialogue', status:'review', revised:true, source:"We used to send weather reports. Now we send names. People listening for someone who hasn't come home.", translation:'Autrefois, nous transmettions la météo. À présent, nous transmettons des noms.'},
  {beat:'Station History', speaker:'Player', kind:'Reply 1', destination:'Station Answer', status:'empty', source:'Do you ever get an answer?', translation:''},
  {beat:'Station History', speaker:'Player', kind:'Reply 2', destination:'Relay Desk', status:'done', source:'Something else I wanted to ask.', translation:'Je voulais vous demander autre chose.'},
  {beat:'Station Answer', speaker:'Mara', kind:'Dialogue', destination:'Station History', status:'done', source:'Sometimes. Not always the answer they wanted.', translation:'Parfois. Pas toujours la réponse attendue.'},
  {beat:'Relay Desk', speaker:'Mara', kind:'Dialogue', status:'done', source:"If you're here about the transmitter, take a number. If you're here about the smoke, take two.", translation:'Pour le transmetteur, prenez un numéro. Pour la fumée, prenez-en deux.'},
  {beat:'Relay Desk', speaker:'Player', kind:'Reply 1', destination:'Station History', status:'done', source:'What happened to this place?', translation:"Que s’est-il passé ici ?"}
];
const labels={review:'Needs review',empty:'Untranslated',done:'Reviewed'};
const drafts=entries.map(e=>e.translation);
const reviewed=entries.map(e=>e.status==='done');
const sourceDrafts=entries.map(e=>e.source);
let selected=0,mode='write',localised=false,external=false;
const diskText='Nous transmettions autrefois des bulletins météo. À présent, ce sont des noms. Ceux de personnes qui ne sont pas rentrées.';
const announce=text=>{$('announcement').textContent=text;};
function element(tag,text,cls){const node=document.createElement(tag);if(text!==undefined)node.textContent=text;if(cls)node.className=cls;return node;}
function button(text,action,cls){const node=element('button',text,cls);node.onclick=action;return node;}
function dirty(index){return drafts[index]!==entries[index].translation||reviewed[index]!== (entries[index].status==='done');}
function resize(field){field.style.height='auto';field.style.height=`${Math.min(340,Math.max(48,field.scrollHeight+2))}px`;}
function focusPassage(){const field=$(mode==='write'?`source-${selected}`:`translation-${selected}`);if(field){field.focus();field.scrollIntoView({block:'nearest'});}}
function highlight(index){selected=index;document.querySelectorAll('[data-passage]').forEach(row=>row.classList.toggle('active-passage',Number(row.dataset.passage)===index));}
function renderNav(){
  $('beat-nav').replaceChildren();
  ['Relay Desk','Station History','Station Answer'].forEach(beat=>{
    const node=button(beat,()=>selectEntry(entries.findIndex(e=>e.beat===beat)));
    node.setAttribute('aria-current',String(entries[selected].beat===beat));$('beat-nav').append(node);
  });
}
function entryContext(container,beat){
  const incoming=beat==='Station History'?5:beat==='Station Answer'?1:null;
  if(incoming===null){container.append(element('p','Scene start · the player approaches the relay desk.','arrival'));return;}
  const details=element('details',undefined,'arrival');
  details.append(element('summary',`From ${entries[incoming].beat} · “${entries[incoming].source}”`));
  details.append(element('p',entries.find(e=>e.beat===entries[incoming].beat&&e.kind==='Dialogue').source,'prose'));
  container.append(details);
}
function renderManuscript(container,translation){
  container.replaceChildren();const beat=entries[selected].beat;entryContext(container,beat);
  if(translation){const headings=element('div',undefined,'language-headings');headings.append(element('span','Source · English'),element('span','French'));container.append(headings);}
  let repliesStarted=false;
  entries.forEach((entry,index)=>{
    if(entry.beat!==beat)return;
    if(entry.kind.startsWith('Reply')&&!repliesStarted){container.append(element('h3','Replies','replies-heading'));repliesStarted=true;}
    const row=element('section',undefined,'passage');row.dataset.passage=index;
    const source=element('div',undefined,'source-passage');source.append(element('p',`${entry.kind} · ${entry.speaker}`,'speaker'));
    if(translation){source.append(element('p',sourceDrafts[index],'prose'));}
    else {
      const label=element('label',`${entry.kind} · ${entry.speaker}`,'sr-only');label.htmlFor=`source-${index}`;source.append(label);
      const field=element('textarea',undefined,'prose source-field');field.id=`source-${index}`;field.value=sourceDrafts[index];field.rows=1;
      field.onfocus=()=>highlight(index);field.oninput=()=>{sourceDrafts[index]=field.value;resize(field);announce('Source draft kept in memory · sample catalogue unchanged');};source.append(field);
    }
    if(entry.destination)source.append(button(`→ ${entry.destination}`,()=>selectEntry(entries.findIndex(e=>e.beat===entry.destination)),'destination'));
    row.append(source);
    if(translation){
      const target=element('div',undefined,'target-passage');
      const state=element('p',undefined,'translation-state');state.id=`status-${index}`;target.append(state);
      if(entry.revised){const details=element('details',undefined,'revision');details.append(element('summary','Source revised · compare'),element('p','Previously: “We used to send weather reports. Now we send names.”'));target.append(details);}
      const label=element('label',`French translation · ${entry.kind} · ${entry.beat}`,'sr-only');label.htmlFor=`translation-${index}`;target.append(label);
      const field=element('textarea',undefined,'prose translation-field');field.id=`translation-${index}`;field.value=drafts[index];field.placeholder='Write a translation…';field.lang='fr';field.rows=1;
      field.onfocus=()=>highlight(index);field.oninput=()=>{drafts[index]=field.value;reviewed[index]=false;resize(field);updateState(index);};
      field.onkeydown=event=>{if((event.ctrlKey||event.metaKey)&&event.key==='s'){event.preventDefault();save(index);}};target.append(field);
      const footer=element('div',undefined,'translation-footer');
      const reviewLabel=element('label',undefined,'review-state');const check=element('input');check.type='checkbox';check.id=`review-${index}`;check.onchange=()=>{reviewed[index]=check.checked;updateState(index);};reviewLabel.append(check,document.createTextNode('Reviewed'));footer.append(reviewLabel);
      const persistence=element('span',undefined,'muted');persistence.id=`saved-${index}`;footer.append(persistence);
      const saveButton=button('Save',()=>save(index),'primary');saveButton.id=`save-${index}`;footer.append(saveButton);target.append(footer);row.append(target);
    }
    container.append(row);
  });
  if(translation&&beat==='Station History'){
    const notes=element('details',undefined,'notes');notes.append(element('summary','Translator notes'),element('p','“Names” refers to missing people. Keep the short, matter-of-fact delivery. (Illustrative note.)'));container.append(notes);
  }
}
function updateState(index){
  if(!$(`translation-${index}`))return;
  const changed=dirty(index),sourceChanged=sourceDrafts[index]!==entries[index].source;
  $(`status-${index}`).textContent=sourceChanged?'Source draft changed · catalogue refresh pending':!drafts[index].trim()?'Untranslated':reviewed[index]?(changed?'Review pending save':'Reviewed'):'Needs review';
  $(`saved-${index}`).textContent=changed?'Unsaved changes':drafts[index].trim()?'Saved':'';
  $(`save-${index}`).hidden=!changed;$(`review-${index}`).checked=reviewed[index];$(`review-${index}`).disabled=!drafts[index].trim()||sourceChanged;
}
function render(){
  $('write-title').textContent=entries[selected].beat;$('entry-title').textContent=entries[selected].beat;
  renderManuscript($('write-manuscript'),false);renderManuscript($('beat-manuscript'),true);
  entries.forEach((_,index)=>updateState(index));renderNav();renderQueue();highlight(selected);
  requestAnimationFrame(()=>document.querySelectorAll('section:not([hidden]) textarea').forEach(field=>{if(field.offsetWidth)resize(field);}));
}
function selectEntry(index){selected=index;render();focusPassage();announce(`${entries[index].beat} · ${entries[index].kind}`);}
function renderQueue(){
  $('entries').replaceChildren();const query=$('search').value.toLowerCase();let count=0;
  entries.forEach((entry,index)=>{
    if(($('filter').value!=='all'&&entry.status!==$('filter').value)||!`${sourceDrafts[index]} ${drafts[index]} ${entry.beat}`.toLowerCase().includes(query))return;
    count++;const row=button('',()=>{$('queue-dialog').close();selectEntry(index);},'queue-entry');row.append(element('span',sourceDrafts[index]),element('small',`${entry.beat} · ${labels[entry.status]}${dirty(index)?' · unsaved changes':''}`));$('entries').append(row);
  });
  if(!count)$('entries').append(element('p','No matching passages. Try another filter or search.'));
  $('coverage').textContent=`· ${entries.filter(e=>e.status!=='done').length} need attention`;
}
function showMode(next){
  mode=next;$('writing').hidden=mode!=='write';$('setup').hidden=mode!=='locale'||localised;$('catalogue').hidden=mode!=='locale'||!localised;
  $('write-tab').setAttribute('aria-pressed',String(mode==='write'));$('locale-tab').setAttribute('aria-pressed',String(mode==='locale'));$('external').disabled=!localised;
  render();announce(localised?'Connected sample · changes stay in memory':'Initial draft · source text only');
}
function save(index){
  if(external){$('compare').click();return;}
  entries[index].translation=drafts[index];entries[index].status=!drafts[index].trim()?'empty':reviewed[index]?'done':'review';
  if(reviewed[index])entries[index].revised=false;
  updateState(index);renderQueue();$(`translation-${index}`).focus();announce(reviewed[index]?'Translation and review saved in sample':'Translation draft saved in sample');
}
$('write-tab').onclick=()=>{showMode('write');focusPassage();};$('locale-tab').onclick=()=>{showMode('locale');if(localised)focusPassage();};
$('connect').onclick=()=>{localised=true;document.querySelector('[value=localised]').checked=true;showMode('locale');};
document.querySelectorAll('[name=scenario]').forEach(input=>input.onchange=()=>{localised=input.value==='localised';showMode(mode);});
$('theme').onclick=()=>{const dark=document.body.classList.toggle('dark');$('theme').setAttribute('aria-pressed',String(dark));};
$('queue-open').onclick=()=>{renderQueue();$('queue-dialog').showModal();};$('queue-close').onclick=()=>$('queue-dialog').close();
$('filter').onchange=renderQueue;$('search').oninput=renderQueue;
$('files').onclick=()=>$('file-dialog').showModal();
$('external').onclick=()=>{external=true;selected=0;showMode('locale');$('external-notice').hidden=false;announce('Simulated external change · compare before saving');};
$('compare').onclick=()=>{$('your-version').textContent=drafts[0]||'(Empty draft)';$('disk-version').textContent=diskText;$('conflict-dialog').showModal();};
$('close-conflict').onclick=()=>$('conflict-dialog').close();
function resolve(useDisk){external=false;$('external-notice').hidden=true;entries[0].translation=diskText;if(useDisk)drafts[0]=diskText;entries[0].status='review';reviewed[0]=false;$('conflict-dialog').close();render();announce(useDisk?'File version loaded for review':'Draft retained · save when ready');}
$('keep').onclick=()=>resolve(false);$('use-disk').onclick=()=>resolve(true);
window.addEventListener('resize',()=>document.querySelectorAll('textarea').forEach(field=>{if(field.offsetWidth)resize(field);}));
showMode('write');
