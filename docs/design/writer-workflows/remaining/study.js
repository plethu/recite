/* Disposable, file-openable interaction study. No compiler, runtime or file I/O. */
'use strict';
const $ = id => document.getElementById(id);
const esc = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const shortcut = /Mac/i.test(navigator.platform) ? '⌘ ↵' : 'Ctrl ↵';
const state = {
  variant:'default', forms:{default:['Vous avez {count} billet.','Vous avez {count} billets.'],formal:['','']},
  saved:{}, reviewed:{default:false,formal:false}, dirty:{default:false,formal:false}, metadata:true,
  threshold:3, appliedThreshold:3, any:'any', appliedAny:'any', extra:false, appliedExtra:false,
  effects:[{name:'play_sfx',arg:'gate_latch',mode:'Immediate'},{name:'mark_map',arg:'floodgate',mode:'Blocking'},{name:'advance_thread',arg:'courier_route',mode:'Deferred'}],
  appliedEffects:null, revision:12, catalogueRevision:3, schemaRevision:1, schemaAttempt:0, rulesDirty:false, complex:false,
  inputs:{standing:4,key:true,permit:false,count:2,locale:'fr-CA',variant:'formal',drafts:false},
  run:null, owner:'standalone', schemaPhase:'current',  declarations:{has_item:{description:'Whether the player carries the gate key.',argument:'item',type:'ItemId'},mark_map:{description:'Ask the game to mark a location on the map.',argument:'location',type:'LocationId'},Mara:{description:'The relay station attendant.',argument:'display_name',type:'string'}},
  source:':: relay_desk\n> departure@11111111111111111111 speaker=mara\n  The last tram leaves at eight.\n-> missing_gate\n:: ticket_desk\n> tickets@44444444444444444444 speaker=mara\n  Here you are.\n-> relay_desk\n',
  completion:false, sourceDirty:false, buildPhase:'idle', buildTarget:'relay', builtTarget:null, buildAttempt:0, failBuild:false,
  conflict:'none', resolution:null, editor:'System default', watching:true, fileDirty:true
};
state.selectedEffect = 'mark_map';
state.appliedSource = state.source;
state.saved = structuredClone(state.forms);
state.appliedEffects = structuredClone(state.effects);
function page(){ return location.hash.slice(1).split('/')[0] || 'localise'; }
function heading(eyebrow,title,tools=''){return `<div class="heading"><div><p class="eyebrow">${eyebrow}</p><h1>${title}</h1></div><div class="page-tools">${tools}</div></div>`;}
function button(action,label,primary=false,disabled=false){return `<button type="button" data-act="${action}"${primary?' class="primary"':''}${disabled?' disabled':''}>${label}</button>`;}
function primary(label,disabled=false){return `<button id="primary-submit" class="primary" type="submit"${disabled?' disabled':''}>${label}<kbd>${shortcut}</kbd></button>`;}
function options(values,selected){return values.map(([v,label])=>`<option value="${v}"${v===selected?' selected':''}>${label}</option>`).join('');}
function notice(text,actions=''){return `<div class="notice"><p>${text}</p>${actions}</div>`;}
function announce(text){ $('status').textContent=text; }
function go(route){location.hash=route;}
function retainFocus(id){render(false); if(id) $(id)?.focus();}
function localise(){
 const v=state.variant, forms=state.forms[v], other=v==='default'?'formal':'default';
 return heading('Relay Hub / Tickets · Mara','Translate tickets','<a href="#preview">Try passage →</a>')+`
 <div class="toolbar"><a href="#files">French · locale/fr.po</a><details><summary>Conversation & notes</summary><p class="prose">Player: There are two of us. Can we still get on?<br>Mara: Keep them dry. The ink runs.</p><p>Writer’s note: these are paper tram tickets.</p></details></div>
 <section class="translation-work"><div class="source-copy"><span class="metadata">Source · English</span><p class="prose">You have {count} ticket.<br>You have {count} tickets.</p></div>
 <form id="translation"><div class="inline-fields"><label>Wording<select id="variant">${options([['default','Default wording'],['formal','Formal']],v)}</select></label><span class="metadata">${other==='formal'?'Formal':'Default'} · ${state.dirty[other]?'unsaved edits':state.saved[other].every(Boolean)?'saved':'not translated'}</span></div>
 ${!state.metadata?notice('Plural rule missing. Restore it before saving.',button('repair-metadata','Use French rule in sample')):''}
 <div class="form-grid">${forms.map((value,i)=>`<label class="form-label">${i===0?'For 0 or 1 ticket':'For 2 or more tickets'}<textarea id="form-${i}" data-form="${i}" aria-describedby="translation-help">${esc(value)}</textarea></label>`).join('')}</div>
 <div class="inline-fields"><p id="translation-help" class="metadata">Keep <code>{count}</code> in each translation.</p><details><summary>Plural rule</summary><p><code>nplurals=2; plural=(n &gt; 1)</code></p><p>Form 1: msgstr[0] · Form 2: msgstr[1]</p></details></div>
 <p id="translation-error" class="error" hidden></p><label class="check"><input id="reviewed" type="checkbox" ${state.reviewed[v]?'checked':''}>Both forms reviewed</label>
 <div class="actions action-bar">${primary('Save wording',!state.metadata)}${button('discard-translation','Discard edits',false,!state.dirty[v])}<span id="translation-state" class="metadata">${state.dirty[v]?'Unsaved changes':state.reviewed[v]?'Reviewed · saved':'Needs review'}</span></div></form></section>`;
}
function rules(){return heading('Relay Hub / Relay Desk · Reply 1','Reply rules','<a href="#preview">Try reply →</a>')+`
 <div class="context"><p class="metadata">Mara: The gate is locked. Who sent you?</p><p class="prose">Let me through. I’m carrying the relay parts.</p></div>
 <form id="rules-form" class="rule-work"><section><h2>Available when</h2>
 <fieldset class="branch"><legend>All of these match</legend><label class="sentence">Standing is at least <input id="threshold" type="number" min="0" max="100" value="${state.threshold}" required></label>
 <fieldset class="nested"><legend><label>And <select id="group-mode">${options([['any','any of these match'],['all','all of these match']],state.any)}</select></label></legend><p>Player has <strong>gate key</strong></p><p>Player has <strong>courier permit</strong></p></fieldset>
 ${state.extra?'<p>Courier thread is not completed</p>':''}${button('toggle-condition',state.extra?'Remove thread condition':'+ Thread condition')}</fieldset>
 ${state.complex?notice('This condition needs the source editor.','<a href="#source">Edit expression →</a>'):''}</section>
 <section><h2>On choosing this reply</h2><ol class="effects">${state.effects.map((e,i)=>`<li><div class="effect-row"><button type="button" class="effect-select" data-effect="${e.name}" aria-expanded="${state.selectedEffect===e.name}"><strong>${({play_sfx:'Play sound',mark_map:'Mark map',advance_thread:'Advance thread'})[e.name]}</strong><small>${e.arg} · ${e.mode}</small></button><button type="button" data-up="${i}" aria-label="Move ${e.name} earlier" ${i===0?'disabled':''}>↑</button><button type="button" data-down="${i}" aria-label="Move ${e.name} later" ${i===state.effects.length-1?'disabled':''}>↓</button></div>${state.selectedEffect===e.name?`<div class="effect-editor"><div class="inline-fields"><label>${e.name==='mark_map'?'Map location':e.name==='play_sfx'?'Sound':'Thread'}<input value="${esc(e.arg)}" id="effect-arg" spellcheck="false"></label><label>Continue<select id="effect-mode">${options([['Immediate','Immediately'],['Blocking','After acknowledgement'],['Deferred','Send at scene end']],e.mode)}</select></label></div><a class="metadata" href="#schema-effect">${e.name} · View declaration</a></div>`:''}</li>`).join('')}</ol></section>
 <div class="actions action-bar">${primary('Apply changes',!state.rulesDirty)}${button('discard-rules','Discard edits',false,!state.rulesDirty)}<span id="rules-state" class="metadata">${state.rulesDirty?'Unapplied changes':'Up to date'}</span></div></form>`;}

function preview(){
 const r=state.run, stale=r&&(r.revision!==state.revision||r.catalogueRevision!==state.catalogueRevision||r.schemaRevision!==state.schemaRevision);
 return heading('Relay Hub / Tickets and Relay Desk','Try this scene','<a href="#rules">Return to reply</a><a href="#localise">Return to translation</a>')+
 `<div class="workspace"><section>
 ${r?`<p class="metadata">Run ${r.number} · source revision ${r.revision} · ${r.drafts?'includes translation drafts':'saved translations only'}</p>`:'<p class="muted">No preview started.</p>'}
 ${stale?notice('Source, declarations or translations changed after this run started. You are still seeing the previous revision.',button('start-preview','Restart with current source',true)):''}
 <div class="run"><h2>${r?'Delivered passage':'Passage preview'}</h2><p class="result">${r?esc(r.text):'Start a preview to inspect delivered text and the available reply.'}</p>${r?`<p class="metadata">${esc(r.provenance)}</p>${r.trace.some(t=>t.includes('no entry')||t.includes('variant missing'))?'<p class="metadata">Using fallback wording · expand the trace for details.</p>':''}<h2>Reply</h2>${r.allowed?button('choose-reply','Let me through. I’m carrying the relay parts.',true,!!r.chosen):'<p class="empty-rule">Unavailable in this test: the condition did not pass.</p>'}`:''}
 ${r?.waiting?notice(`Waiting for acknowledgement · <code>${esc(r.waiting.name)}(${esc(r.waiting.arg)})</code>`,button('ack','Acknowledge completed')+button('fail-effect','Report failure')):''}
 ${r?.ended?'<p>Scene ended. Deferred requests are listed in the trace.</p>':''}</div>
 <details><summary>Why this text and reply?</summary><ol class="trace">${r?r.trace.map(t=>`<li>${esc(t)}</li>`).join(''):'<li>No events yet.</li>'}</ol></details>
 </section>
 <aside class="aside input-panel"><h2>Game state</h2><label>Standing<input id="standing" type="number" min="0" value="${state.inputs.standing}"></label><p><label><input id="key" type="checkbox" ${state.inputs.key?'checked':''}>Has gate key</label><label><input id="permit" type="checkbox" ${state.inputs.permit?'checked':''}>Has courier permit</label></p><h2>Language & wording</h2><label>Ticket count<input id="count" type="number" min="0" value="${state.inputs.count}"></label><label>Dialogue locale<select id="locale">${options([['source','Source text only'],['fr-CA','French (Canada) · fr-CA'],['fr','French · fr']],state.inputs.locale)}</select></label><label>Requested variant<select id="preview-variant">${options([['default','Default'],['formal','Formal']],state.inputs.variant)}</select></label><p><label><input id="drafts" type="checkbox" ${state.inputs.drafts?'checked':''}>Include unsaved translations</label></p><p class="metadata">Changes take effect on Restart.</p><div class="actions">${button('start-preview',r?'Restart preview':'Start preview',true)}${button('reset-inputs','Reset')}</div><a href="#schema">Input declarations</a></aside></div>`;
}
function schemaKey(){return location.hash.includes('schema-effect')?'mark_map':location.hash.includes('schema-speaker')?'Mara':'has_item';}
function schema(){const key=schemaKey(),declaration=state.declarations[key];return heading('Project / Declarations','Declarations','<a href="#rules">Return to reply →</a>')+
 `<div class="list-detail"><nav aria-label="Declarations"><a href="#schema" ${key==='has_item'?'aria-current="page"':''}>has_item<br><small>Condition · boolean</small></a><a href="#schema-effect" ${key==='mark_map'?'aria-current="page"':''}>mark_map<br><small>Effect · location</small></a><a href="#schema-speaker" ${key==='Mara'?'aria-current="page"':''}>Mara<br><small>Speaker</small></a></nav><section>
 <h2>${key}</h2><p>${esc(declaration.description)}</p><p><code>${key}(${esc(declaration.argument)}: ${esc(declaration.type)})${key==='has_item'?' → boolean':''}</code></p>
 <details class="provenance"><summary>Defined in ${state.owner==='standalone'?'schema/dialogue.toml':state.owner==='engine'?'game/dialogue/schema.rs':'an unknown source'}</summary><p>Generated output: <code>generated/dialogue.schema.json</code> · ${state.schemaPhase}</p><a href="#rules">Used in Relay Desk →</a></details>
 ${state.schemaPhase==='failed'?notice('Generation failed. The last usable manifest is retained; it does not include your source changes.',button('generate','Retry generation',true)):state.schemaPhase==='stale'?notice('The declaration source is newer than the manifest. Completion and preview still use the last generated version.',button('generate','Regenerate',true)):state.schemaPhase==='generating'?jobStatus('Generating declarations','Previous output stays available.'):''}
 ${state.owner==='standalone'?`<form id="schema-form"><h2>Edit declaration</h2><label>Description<textarea id="schema-description">${esc(declaration.description)}</textarea></label><div class="inline-fields"><label>${key==='Mara'?'Display name field':'Argument name'}<input id="argument-name" value="${esc(declaration.argument)}" required></label><label>Type<select id="argument-type">${options([['ItemId','Item registry'],['LocationId','Location registry'],['string','Text'],['int','Integer']],declaration.type)}</select></label></div><p class="metadata">${key==='has_item'?'Returns boolean · used as a pure query':key==='mark_map'?'Emits a typed request · delivery mode belongs to each effect statement':'Speaker metadata · independent from dialogue prose'}</p><div class="actions">${primary('Save & regenerate',state.schemaPhase==='generating')}</div></form>`:state.owner==='engine'?`<div class="actions">${button('open-declaration','Open source declaration')}${['failed','stale'].includes(state.schemaPhase)?'':button('generate','Regenerate',true,state.schemaPhase==='generating')}</div>`:'<p class="metadata">Read-only · this source does not provide editing or generation.</p>'}
</section></div>`;}
function source(){const broken=state.source.includes('missing_gate');return heading('Relay Hub','Source','<a href="#rules">Return to manuscript →</a>')+`
 <nav class="subnav"><a href="#source" aria-current="page">relay_hub.recite${state.sourceDirty?' · edited':''}</a><a href="#source-other">floodgate.recite</a></nav>
 <form id="source-form" class="editor-work"><label class="metadata" for="source-text">relay_hub.recite</label><div class="editor-container"><textarea class="source-editor" id="source-text" spellcheck="false">${esc(state.source)}</textarea>
 ${state.completion?`<div class="completion" role="region" aria-label="Destination suggestions">${button('insert-destination','floodgate <small>floodgate.recite · destination</small>')}</div>`:''}</div>
 <div class="actions">${button('complete','Complete · Ctrl+Space')}${button('rename','Rename beat')}</div>
 <h2>Problems <span class="metadata">${broken?'1':'0'}</span></h2><ul class="diagnostic-list">${broken?`<li>${button('jump-diagnostic','Unknown destination “missing_gate”<small>relay_hub.recite:4</small>')}<a href="#source-other">Inspect floodgate →</a></li>`:'<li class="metadata">No problems found.</li>'}</ul>
 <div class="actions action-bar">${primary('Apply draft',!state.sourceDirty)}${button('discard-source','Discard edits',false,!state.sourceDirty)}</div></form>`;}

function sourceOther(){return heading('Relay Hub','Source','<a href="#rules">Return to manuscript →</a>')+`<nav class="subnav"><a href="#source">relay_hub.recite${state.sourceDirty?' · edited':''}</a><a href="#source-other" aria-current="page">floodgate.recite</a></nav><section class="editor-work"><label class="metadata" for="referenced-source">floodgate.recite · line 1 · read-only reference</label><textarea readonly id="referenced-source" class="source-editor">:: floodgate
> gate@33333333333333333333 speaker=mara
  Keep the latch down until the tram passes.
-> END</textarea><a href="#source">← Back to problem in relay_hub.recite</a></section>`;}

function rename(){const old=state.source.match(/^:: (w+)/)?.[1]||'relay_desk';return heading('relay_hub.recite','Rename beat','<a href="#source">Cancel</a>')+`<form id="rename-form"><label class="rename-field">New name<input id="rename-value" value="${esc(state.renameValue||'relay_counter')}" pattern="[a-z][a-z0-9_]*" required></label><div id="rename-comparison">${renameComparison(old)}</div><div class="actions action-bar">${primary('Apply rename')}<span class="metadata">2 references · stable line IDs unchanged</span></div></form>`;}
function renameComparison(old){const name=state.renameValue||'relay_counter';return comparisonView('Current source','Proposed source',[{label:'Line 1 · declaration',before:`:: ${old}`,after:`:: ${name}`},{label:'Line 8 · return',before:`-> ${old}`,after:`-> ${name}`}],'relay_hub.recite',{unified:state.unified});}
function build(){const running=state.buildPhase==='running';return heading('Project / Build scenes','Build scenes','<a href="#rules">Return to writing →</a>')+
 `<div class="narrow"><form id="build-form"><label>Scene selection<select id="build-target" ${running?'disabled':''}>${options([['relay','Relay service · 2 scenes'],['floodgate','Floodgate encounter · 1 scene']],state.buildTarget)}</select></label><h2>Output</h2><p><code>build/${state.buildTarget}.recitec</code></p><details><summary>Included files & declarations</summary><dl><dt>Manifest</dt><dd><code>recite.project.toml</code></dd><dt>Scenes</dt><dd>${state.buildTarget==='relay'?'relay_hub, last_tram':'floodgate'}</dd><dt>Source</dt><dd>${state.buildTarget==='relay'?'relay_hub.recite, last_tram.recite':'floodgate.recite'}</dd><dt>Schema</dt><dd>Last generated declarations · ${state.schemaPhase}</dd><dt>Output</dt><dd><code>build/${state.buildTarget}.recitec</code></dd></dl></details>
 ${state.fileDirty?notice('There are unsaved changes.'):''}
 <div class="actions">${primary(state.fileDirty?'Save and build':'Build scenes',running)}${state.fileDirty?button('build-saved','Build saved version',false,running):''}</div>
</form><section class="build-result" aria-label="Build status">${running?jobStatus('Building scenes','Validating selected source…',button('cancel-build','Cancel')):''}
 <p id="build-state">${{idle:'Ready to build.',running:'The previous successful output remains available.',cancelled:'Build cancelled. Previous output retained.',failed:'Build failed. Previous output retained.',done:`Build complete · ${state.builtTarget}`}[state.buildPhase]}</p>
 ${state.buildPhase==='failed'?notice('Unknown destination in the selected source. Correct it and retry the same scene selection.','<a href="#source">Open diagnostic →</a>'):''}</section>
</div>`;}
function files(){return heading('Project','Files & external editor','<a href="#localise">Return to translation →</a>')+`<div class="narrow"><section><h2>Open catalogue</h2><div class="file-row"><div><code>locale/fr.po</code><small>${state.conflict==='detected'?'Changed outside Recite':'No external changes detected'}</small></div><a href="#compare">Compare versions →</a></div>${state.conflict==='detected'?notice('The disk version changed. Compare before saving.'):''}</section><section><h2>External editor</h2><label>Open with<select id="external-editor">${options([['System default','System default'],['PO editor','Registered PO editor']],state.editor)}</select></label><div class="actions">${button('save-open','Save then open',true)}${button('open-external','Open disk version')}</div></section><details><summary>File watching</summary><label class="check"><input id="watching" type="checkbox" ${state.watching?'checked':''}>Watch open files for changes</label><p class="metadata">Saving always checks for external changes.</p>${button('refresh-files','Check now')}</details></div>`;}

function compare(){return heading('locale/fr.po · Formal wording · Tickets','Review external changes','<a href="#localise">Return to translation</a>')+comparisonView('Your draft · unsaved','Disk · external edit',state.forms.formal.map((value,i)=>({label:i===0?'0 or 1 ticket':'2 or more tickets',before:value,after:diskForms[i]})),'Tickets · formal',{unified:state.unified})+`<form id="resolve-form"><fieldset class="resolution"><legend>Continue editing from</legend><label class="choice"><input name="resolution" type="radio" value="mine" ${state.resolution==='mine'?'checked':''}><span><strong>Your draft</strong><small>Keep both forms as written here</small></span></label><label class="choice"><input name="resolution" type="radio" value="disk" ${state.resolution==='disk'?'checked':''}><span><strong>Disk version</strong><small>Bring both external forms into the editor</small></span></label></fieldset><div class="actions action-bar">${primary('Use as editable draft',!state.resolution)}<span class="metadata">Review and save in the translation editor.</span></div></form>`;}
function updates(){return heading('locale/fr.po · Relay Hub','Review source updates','<a href="#localise">Return to translation</a>')+comparisonView('Previous source','Current source',[
 {label:'The last tram',kind:'Changed',before:'The last tram leaves at nine.',after:'The last tram leaves at eight. Don’t wait for me.'},
 {label:'A place on the tram',kind:'Added',before:'',after:'There is room for one more.'},
 {label:'The old timetable',kind:'Removed',before:'The old timetable hangs beside the door.',after:''}
 ],'3 passages',{unified:state.unified,unchanged:'Mara: Keep your ticket. You’ll need it at the next stop.'})+`<details><summary>Existing French translation · The last tram</summary><p class="prose">Le dernier tram part à neuf.</p></details><p class="metadata">Translations stay intact. Changed text needs review; removed entries stay in history.</p><div class="actions action-bar">${button('update-sources',state.sourcesUpdated?'Catalogue updated':'Update catalogue · 3 entries',true,!!state.sourcesUpdated)}<a href="#localise">Leave without updating</a></div>`;}
const diskForms=['Vous disposez de {count} titre de transport.','Vous disposez de {count} titres de transport.'];
const screens={updates,localise,rules,preview,schema,source,'schema-effect':schema,'schema-speaker':schema,'source-other':sourceOther,rename,build,files,compare};

function render(focus=true){const active=document.activeElement;const restore=$('main').contains(active);const activeId=active?.id;const action=active?.dataset?.act;const key=page();$('main').innerHTML=(screens[key]||localise)();$('screen').value=key.startsWith('schema')?'schema':key==='source-other'||key==='rename'?'source':key==='compare'?'files':key;document.querySelectorAll('[data-mode]').forEach(a=>{if(a.dataset.mode===(key==='localise'||key==='compare'||key==='updates'?'localise':'write'))a.setAttribute('aria-current','page');else a.removeAttribute('aria-current');});if(focus){$('main').focus();announce('Sample workspace · changes last until reload');}else if(restore){const next=activeId?$(activeId):action?document.querySelector(`[data-act="${action}"]`):null;if(next&&!next.disabled)next.focus();else $('main').focus();}}
state.savedReviewed={default:false,formal:false};
state.formalPublished=false;
function markRules(){state.rulesDirty=true;if($('rules-state'))$('rules-state').textContent='Unapplied rule changes';document.querySelectorAll('#rules-form button[type=submit], #rules-form [data-act=discard-rules]').forEach(b=>b.disabled=false);}
function startPreview(){
 const i=state.inputs;
 if(!Number.isInteger(i.count)||i.count<0||!Number.isInteger(i.standing)||i.standing<0){announce('Use non-negative whole numbers for preview inputs.');return;}
 if(i.locale!=='source'&&!state.metadata){announce('Repair the catalogue plural rule before previewing this locale.');return;}
 const trace=[`Source snapshot: revision ${state.revision}. Inputs are fixed for this run.`];
 let text,provenance,variant=i.variant;
 if(i.locale==='source'){
   text=i.count===1?'You have {count} ticket.':'You have {count} tickets.';provenance='Source text · English source-form rule';trace.push('No dialogue locale requested: translation lookup bypassed.');
 }else{
   if(i.locale==='fr-CA')trace.push('fr-CA: no entry. Try the broader locale fr.');
   const draftFormal=i.drafts&&state.dirty.formal&&state.forms.formal.every(t=>t.trim());
   if(variant==='formal'&&!state.formalPublished&&!draftFormal){variant='default';trace.push('Formal variant missing: use the default entry. This is fallback, not a completed formal translation.');}
   const includeDraft=i.drafts&&state.dirty[variant];
   const form=i.count>1?1:0;
   text=(includeDraft?state.forms:state.saved)[variant][form];
   provenance=`fr · ${variant} wording · form ${form+1} · ${includeDraft?'unsaved draft':'saved catalogue'}`;
   trace.push(`fr catalogue rule selects form ${form+1} for count ${i.count}.`);
 }
 const allowed=i.standing>=state.appliedThreshold&&(state.appliedAny==='any'?(i.key||i.permit):(i.key&&i.permit));
 trace.push(`Standing ${i.standing} ≥ ${state.appliedThreshold}: ${i.standing>=state.appliedThreshold?'true':'false'}. ${state.appliedAny==='any'?'Any':'All'} of key / permit: ${state.appliedAny==='any'?i.key||i.permit:i.key&&i.permit}.`);
 if(state.appliedExtra)trace.push('Thread courier_route completed: false. Negated condition: true.');
 state.run={number:(state.run?.number||0)+1,revision:state.revision,catalogueRevision:state.catalogueRevision,schemaRevision:state.schemaRevision,drafts:i.drafts,text:text.replaceAll('{count}',String(i.count)),provenance,allowed,trace,chosen:false,waiting:null,ended:false,effects:structuredClone(state.appliedEffects),cursor:0,deferred:[]};
 announce('Preview restarted with the displayed inputs.');retainFocus('main');
}
function advanceRequests(){const r=state.run;while(r.cursor<r.effects.length){const effect=r.effects[r.cursor++];if(effect.mode==='Deferred'){r.deferred.push(effect);r.trace.push(`Deferred request collected: ${effect.name}(${effect.arg}).`);}else{r.trace.push(`${effect.mode} request: ${effect.name}(${effect.arg}). No game action executed.`);if(effect.mode==='Blocking'){r.waiting=effect;return;}}}r.ended=true;r.trace.push(`Scene ended: ${r.deferred.length} deferred request(s) returned to the caller.`);}
function generate(fail=false){const attempt=++state.schemaAttempt;state.schemaPhase='generating';retainFocus('main');announce('Generating sample declarations.');setTimeout(()=>{if(attempt!==state.schemaAttempt)return;if(!fail)state.schemaRevision++;state.schemaPhase=fail?'failed':'current';if(page().startsWith('schema'))render(false);announce(fail?'Generation failed; previous manifest retained.':'Sample declarations regenerated.');},700);}
function startBuild(saveFirst=false){if(state.buildPhase==='running')return;if(saveFirst&&state.conflict!=='none'){announce('Save blocked by the external edit. Compare versions before building.');go('files');return;}if(saveFirst)state.fileDirty=false;const generation=++state.buildAttempt,target=state.buildTarget,fail=state.failBuild;state.buildPhase='running';render(false);announce('Sample build started.');setTimeout(()=>{if(generation!==state.buildAttempt)return;state.buildPhase=fail?'failed':'done';if(!fail)state.builtTarget=target;if(page()==='build')render(false);announce(fail?'Build failed; previous output retained.':'Sample build finished; no artifact written.');},1400);}
function applyRules(){state.appliedThreshold=state.threshold;state.appliedAny=state.any;state.appliedExtra=state.extra;state.appliedEffects=structuredClone(state.effects);state.rulesDirty=false;state.revision++;state.fileDirty=true;announce('Rules applied to the sample source. Existing preview is now out of date.');render(false);}
const actions={
 'update-sources':()=>{state.sourcesUpdated=true;render(false);announce('Sample catalogue updated; translations retained.');},
 'toggle-comparison':()=>{state.unified=!state.unified;render(false);},'next-change':()=>{const nodes=[...document.querySelectorAll('[data-change]')];state.changeIndex=((state.changeIndex??-1)+1)%nodes.length;nodes[state.changeIndex]?.focus();},'build-saved':()=>startBuild(false),
 'missing-metadata':()=>{state.metadata=false;retainFocus('main');},'repair-metadata':()=>{state.metadata=true;announce('Sample French plural rule restored; no translation text changed.');retainFocus('variant');},
 'discard-translation':()=>{state.forms[state.variant]=structuredClone(state.saved[state.variant]);state.reviewed[state.variant]=state.savedReviewed[state.variant];state.dirty[state.variant]=false;retainFocus('variant');},
 'toggle-condition':()=>{state.extra=!state.extra;markRules();render(false);},complex:()=>{state.complex=!state.complex;render(false);},
 'discard-rules':()=>{state.threshold=state.appliedThreshold;state.any=state.appliedAny;state.extra=state.appliedExtra;state.effects=structuredClone(state.appliedEffects);state.rulesDirty=false;retainFocus('threshold');},
 'start-preview':startPreview,'choose-reply':()=>{if(!state.run||!state.run.allowed||state.run.chosen)return;state.run.chosen=true;advanceRequests();announce(state.run.waiting?'Preview paused at a blocking request.':'Scene ended.');render(false);},
 ack:()=>{state.run.trace.push(`Acknowledged completed: ${state.run.waiting.name}.`);state.run.waiting=null;advanceRequests();render(false);announce(state.run.waiting?'Waiting for the next blocking request.':'Scene ended; deferred requests returned.');},
 'fail-effect':()=>{state.run.trace.push(`Reported failure: ${state.run.waiting.name}. This trial is stopped; restart to try again.`);state.run.waiting=null;state.run.ended=true;render(false);announce('Sample trial stopped after the reported failure.');},
 'reset-inputs':()=>{state.inputs={standing:4,key:true,permit:false,count:2,locale:'fr-CA',variant:'formal',drafts:false};render(false);announce('Test inputs reset. Restart to use them.');},
 generate:()=>generate(), 'stale-schema':()=>{state.schemaPhase='stale';render(false);},'fail-schema':()=>generate(true),
 'open-declaration':()=>announce('Would open game/dialogue/schema.rs in its registered editor. This sample launches nothing.'),
 complete:()=>{state.completion=!state.completion;render(false);},rename:()=>go('rename'),
 'insert-destination':()=>{state.source=state.source.replace('missing_gate','floodgate');state.sourceDirty=true;state.completion=false;retainFocus('source-text');},
 'jump-diagnostic':()=>{const input=$('source-text'),offset=state.source.indexOf('missing_gate');input.focus();input.setSelectionRange(Math.max(0,offset),Math.max(0,offset)+12);input.scrollTop=input.scrollHeight;announce('Source editor: line 4, unknown destination selected.');},
 'discard-source':()=>{state.source=state.appliedSource;state.sourceDirty=false;retainFocus('source-text');},
 'cancel-build':()=>{state.buildAttempt++;state.buildPhase='cancelled';render(false);announce('Build cancelled. Previous successful output retained.');},'save-build':()=>startBuild(true),
 external:()=>{state.conflict=state.watching?'detected':'unseen';render(false);announce(state.watching?'External change detected; draft retained.':'Sample external change exists; save-time comparison will protect the draft.');},
 'open-external':()=>announce(`Would open the disk version in ${state.editor}. Unsaved drafts stay in Recite.`),
 'save-open':()=>{if(state.conflict!=='none'){announce('Save blocked; nothing launched. Compare the external version first.');go('compare');}else announce(`Sample save succeeded; would open ${state.editor}. No real editor launched.`);},
 'refresh-files':()=>{if(state.conflict==='unseen'){state.conflict='detected';render(false);}announce(state.conflict==='none'?'No changes found in the sample files.':'External changes still need comparison.');}
};
$('main').addEventListener('click',event=>{
 const control=event.target.closest('button');if(!control||control.disabled)return;
 if(control.dataset.act){actions[control.dataset.act]?.();return;}
 if(control.dataset.effect){state.selectedEffect=control.dataset.effect;render(false);document.querySelector(`[data-effect="${state.selectedEffect}"]`)?.focus();return;}
 const direction=control.dataset.up!==undefined?-1:control.dataset.down!==undefined?1:0;
 if(direction){const index=Number(direction===-1?control.dataset.up:control.dataset.down),next=index+direction;[state.effects[index],state.effects[next]]=[state.effects[next],state.effects[index]];markRules();render(false);document.querySelector(`[data-${direction===-1?'up':'down'}="${next}"]`)?.focus();}
});
$('main').addEventListener('input',event=>{
 const el=event.target;
 if(el.dataset.form!==undefined){state.forms[state.variant][Number(el.dataset.form)]=el.value;state.dirty[state.variant]=true;state.reviewed[state.variant]=false;$('reviewed').checked=false;$('translation-state').textContent='Unsaved translation edits';document.querySelector('[data-act=discard-translation]').disabled=false;}
 if(el.id==='threshold'){state.threshold=Number(el.value);markRules();}
 if(el.id==='effect-arg'){state.effects.find(e=>e.name===state.selectedEffect).arg=el.value;markRules();}
 if(el.id==='rename-value'){state.renameValue=el.value;$('rename-comparison').innerHTML=renameComparison(state.source.match(/^:: (\w+)/)?.[1]||'relay_desk');}
 if(el.id==='source-text'){state.source=el.value;state.sourceDirty=true;document.querySelector('#source-form button[type=submit]').disabled=false;document.querySelector('[data-act=discard-source]').disabled=false;}
 if(el.id==='schema-description')state.declarations[schemaKey()].description=el.value;
 if(el.id==='argument-name')state.declarations[schemaKey()].argument=el.value;
});
document.addEventListener('change',event=>{
 const el=event.target;
 if(el.id==='variant'){state.variant=el.value;retainFocus('variant');}
 if(el.id==='reviewed'){state.reviewed[state.variant]=el.checked;state.dirty[state.variant]=true;$('translation-state').textContent='Review pending save';document.querySelector('[data-act=discard-translation]').disabled=false;}
 if(el.id==='group-mode'){state.any=el.value;markRules();}
 if(el.id==='effect-mode'){state.effects.find(e=>e.name===state.selectedEffect).mode=el.value;markRules();render(false);$('effect-mode').focus();}
 if(['standing','count'].includes(el.id))state.inputs[el.id]=Number(el.value);
 if(['key','permit','drafts'].includes(el.id))state.inputs[el.id]=el.checked;
 if(el.id==='locale')state.inputs.locale=el.value;
 if(el.id==='preview-variant')state.inputs.variant=el.value;
 if(el.id==='owner'){state.owner=el.value;state.schemaPhase='current';state.schemaAttempt++;retainFocus('owner');}
 if(el.id==='argument-type')state.declarations[schemaKey()].type=el.value;
 if(el.id==='build-target'){state.buildTarget=el.value;state.buildPhase='idle';retainFocus('build-target');}
 if(el.id==='fail-build')state.failBuild=el.checked;
 if(el.id==='watching')state.watching=el.checked;
 if(el.id==='external-editor')state.editor=el.value;
 if(el.name==='resolution'){state.resolution=el.value;document.querySelector('#resolve-form button[type=submit]').disabled=false;}
});
$('main').addEventListener('submit',event=>{
 event.preventDefault();const id=event.target.id;
 if(id==='translation'){
   if(!state.metadata)return;
   if(state.conflict!=='none'){go('compare');return;}
   if(state.forms[state.variant].some(value=>!value.trim()||!value.includes('{count}'))){$('translation-error').hidden=false;$('translation-error').textContent='Both forms need text and the {count} placeholder. Nothing saved.';$('form-0').focus();announce('Translation not saved. Check both forms.');return;}
   state.saved[state.variant]=structuredClone(state.forms[state.variant]);state.savedReviewed[state.variant]=state.reviewed[state.variant];state.dirty[state.variant]=false;state.catalogueRevision++;if(state.variant==='formal')state.formalPublished=true;announce('This variant saved in the sample; other variants are unchanged.');render(false);
 }
 if(id==='rules-form')applyRules();
 if(id==='schema-form')generate();
 if(id==='source-form'){state.appliedSource=state.source;state.sourceDirty=false;state.revision++;state.fileDirty=true;announce('Source draft applied in the sample. Existing preview needs a restart.');render(false);}
 if(id==='rename-form'){const old=state.source.match(/^:: (\w+)/)?.[1];const renamed=$('rename-value').value;state.source=state.source.split('\n').map(line=>line===`:: ${old}`?`:: ${renamed}`:line===`-> ${old}`?`-> ${renamed}`:line).join('\n');state.sourceDirty=true;go('source');announce('Both sample references renamed. Source draft retained.');}
 if(id==='build-form')startBuild(state.fileDirty);
 if(id==='resolve-form'&&state.resolution){if(state.resolution==='disk')state.forms.formal=structuredClone(diskForms);state.variant='formal';state.dirty.formal=true;state.reviewed.formal=false;state.conflict='none';state.resolution=null;go('localise');announce('Resolution returned as an unsaved, unreviewed draft.');}
});
document.addEventListener('keydown',event=>{if(event.ctrlKey&&event.code==='Space'&&document.activeElement.id==='source-text'){event.preventDefault();state.completion=true;render(false);document.querySelector('.completion button')?.focus();return;}if(event.key==='Enter'&&(event.ctrlKey||event.metaKey)){const form=$('main').querySelector('form'),submit=form?.querySelector('button[type=submit]');if(form&&submit&&!submit.disabled){event.preventDefault();form.requestSubmit(submit);}}});
$('screen').addEventListener('change',event=>go(event.target.value));
$('theme').addEventListener('click',()=>{document.body.classList.toggle('dark');$('theme').textContent=document.body.classList.contains('dark')?'Light appearance':'Dark appearance';});
$('back').addEventListener('click',()=>history.back());
window.addEventListener('hashchange',()=>render());
render(false);

$('scenarios').addEventListener('click',event=>{const action=event.target.closest('[data-act]')?.dataset.act;if(action)actions[action]?.();});
