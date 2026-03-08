# Phase 2: Server Routes + Toolbox UI Tab

## Context Links
- [Phase 1 — Backend module](./phase-01-backend-toolbox-module.md)
- [Server routes](../../ugate/src/web/server.rs) — route wiring pattern
- [Embedded HTML](../../ugate/src/embedded_index.html) — single-file SPA

## Overview
- **Priority:** High
- **Status:** Pending
- **Description:** Wire API routes in server.rs, add "Toolbox" tab to embedded HTML with tool selector, target input, terminal output area

## Requirements

### Functional
- 2 new routes in server.rs match block
- New "Toolbox" tab in nav bar (after "Routing", before "System")
- Tool selector: dropdown with ping/traceroute/nslookup
- Target input: text field for hostname or IP
- Run button + Stop button
- Terminal-like output area (reuse `.stream` CSS class)
- Clear button to reset output
- Filter WS messages: only show `{"type":"toolbox"}` in toolbox stream

### Non-Functional
- Keep embedded_index.html additions under ~80 lines of JS
- Reuse existing CSS classes (`.card`, `.stream`, `.cf`, `.save-btn`)

## Related Code Files

### Modify
- `ugate/src/web/server.rs` — add 2 route entries (~6 lines)
- `ugate/src/embedded_index.html` — add tab + renderToolbox function (~80 lines JS)

## Implementation Steps

### Step 1: Add routes to server.rs

In the `match (method, url.as_str())` block, add after the maintenance routes (before the `_ =>` fallback):

```rust
// Toolbox
(tiny_http::Method::Post, "/api/toolbox/run") => {
    let body = read_body(&mut request);
    crate::web::toolbox::handle_run(&body, &ws_manager)
}
(tiny_http::Method::Post, "/api/toolbox/stop") => {
    crate::web::toolbox::handle_stop()
}
```

### Step 2: Add Toolbox tab to nav

In the `tabs` array (line ~123), add `['toolbox','Toolbox']` after `'routing'`:

```javascript
const tabs=[['status','...'],['config','...'],['uart','UART'],['network','...'],['routing','...'],['toolbox','Toolbox'],['system','...']];
```

### Step 3: Add render dispatch

In the ternary render chain (line ~106), add toolbox case:

```javascript
pg==='toolbox'?renderToolbox():
```

### Step 4: Add renderToolbox function

Insert before the `connectWS()` function (~line 835). Approximately 70 lines:

```javascript
// --- Toolbox page ---
// State: S.toolbox = {tool:'ping', target:'', lines:[], running:false}
let _tbStreamEl=null,_tbRendered=0;

function renderToolbox(){
  if(!S.toolbox) S.toolbox={tool:'ping',target:'',lines:[],running:false};
  const tb=S.toolbox;

  // Tool selector + target input
  const toolSelect = h('select',{
    style:'padding:6px 10px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.85rem',
    onchange:e=>{tb.tool=e.target.value}
  },
    ...['ping','traceroute','nslookup'].map(t=>
      h('option',{value:t,...(tb.tool===t?{selected:''}:{})},t)
    )
  );

  const targetInput = h('input',{
    type:'text',value:tb.target,placeholder:'hostname or IP',
    style:'flex:1;padding:6px 10px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.85rem',
    oninput:e=>{tb.target=e.target.value},
    onkeydown:e=>{if(e.key==='Enter'&&!tb.running)runTool()}
  });

  const runBtn = h('button',{
    cls:'save-btn',
    style:'margin:0;padding:6px 16px;font-size:.8rem;'+(tb.running?'opacity:.5':''),
    onclick:()=>{if(!tb.running)runTool()}
  }, tb.running?'Running...':'Run');

  const stopBtn = tb.running ? h('button',{
    style:'padding:6px 16px;background:#dc2626;color:white;border:none;border-radius:6px;cursor:pointer;font-size:.8rem;font-weight:700',
    onclick:stopTool
  },'Stop') : null;

  const clearBtn = h('button',{
    style:'padding:4px 12px;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:4px;cursor:pointer;font-size:.75rem',
    onclick:()=>{tb.lines=[];_tbStreamEl=null;_tbRendered=0;render()}
  },'Clear');

  const toolbar = h('div',{style:'display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:8px'},
    toolSelect, targetInput, runBtn, ...(stopBtn?[stopBtn]:[]), clearBtn
  );

  // Stream area — reuse .stream CSS, append-only like UART
  if(_tbStreamEl && S.page==='toolbox'){
    while(_tbRendered < tb.lines.length){
      const d = h('div',{},tb.lines[_tbRendered]);
      _tbStreamEl.append(d);
      _tbRendered++;
    }
    _tbStreamEl.scrollTop = _tbStreamEl.scrollHeight;
  } else {
    _tbStreamEl = h('div',{cls:'stream',style:'flex:1;overflow-y:auto;min-height:300px;max-height:60vh'});
    tb.lines.forEach((l,i)=>{_tbStreamEl.append(h('div',{},l));_tbRendered=i+1});
  }

  return h('div',{cls:'card',style:'display:flex;flex-direction:column'},
    h('h3',{},'Network Diagnostics'),
    toolbar,
    _tbStreamEl
  );
}

async function runTool(){
  const tb=S.toolbox;
  tb.lines=[];_tbStreamEl=null;_tbRendered=0;tb.running=true;render();
  try{
    const r=await fetch('/api/toolbox/run',{method:'POST',body:JSON.stringify({tool:tb.tool,target:tb.target})});
    if(!r.ok){const d=await r.json();tb.lines.push('Error: '+(d.error||'failed'));tb.running=false;render()}
  }catch(e){tb.lines.push('Error: '+e.message);tb.running=false;render()}
}

async function stopTool(){
  try{await fetch('/api/toolbox/stop',{method:'POST'})}catch(_){}
}
```

### Step 5: Handle toolbox WS messages

In the `ws.onmessage` handler (line ~840), add toolbox message routing. Currently the code does:
- `d.type==='status'` -> update status
- else -> push to UART stream

Change to also handle `d.type==='toolbox'`:

```javascript
ws.onmessage=e=>{
  try{
    const d=JSON.parse(e.data);
    if(d.type==='status'){S.status=d;if(S.page==='status')updateStatus()}
    else if(d.type==='toolbox'){
      if(S.toolbox){
        if(d.done){
          S.toolbox.running=false;
          S.toolbox.lines.push('--- done (exit code: '+d.code+') ---');
        } else if(d.line!=null){
          S.toolbox.lines.push(d.line);
        }
        if(S.page==='toolbox')render();
      }
    }
    else{
      // existing UART stream handling...
    }
  }catch(_){}
};
```

## Todo List

- [ ] Add 2 toolbox routes to server.rs match block
- [ ] Add 'toolbox' to tabs array in embedded_index.html
- [ ] Add `renderToolbox()` dispatch in render chain
- [ ] Implement `renderToolbox()`, `runTool()`, `stopTool()` JS functions
- [ ] Add toolbox WS message handling in `ws.onmessage`
- [ ] Add `S.toolbox` initialization to state object
- [ ] Verify `cargo check` passes
- [ ] Deploy and test on device: ping, traceroute, nslookup

## Success Criteria
- Toolbox tab visible in nav bar
- Can select tool, enter target, click Run
- Output streams line-by-line in terminal area
- Stop button kills running process
- Clear button resets output
- Only 1 tool runs at a time (Run button disabled while running)
- No interference with UART stream display

## Risk Assessment
- **HTML file size**: ~80 lines added to 870-line file — acceptable
- **WS message filtering**: `type:"toolbox"` prevents mixing with UART data
- **Render performance**: append-only pattern (same as UART) avoids full re-render
