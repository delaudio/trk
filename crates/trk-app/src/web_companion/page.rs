pub(super) const COMPANION_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<title>trk companion</title>
<style>
:root{color-scheme:dark;--bg:#080b10;--panel:#111722;--line:#263247;--text:#eef4ff;--muted:#8190a8;--cyan:#56e0d4;--violet:#a78bfa;--amber:#ffc857;--red:#ff5e7d}
*{box-sizing:border-box}html,body{margin:0;min-height:100%;background:var(--bg);color:var(--text);font:14px ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}body{padding:clamp(10px,2vw,24px)}
header{display:flex;gap:16px;align-items:center;justify-content:space-between;margin-bottom:12px}.brand{font-size:clamp(20px,3vw,34px);font-weight:900;letter-spacing:.16em}.brand span{color:var(--cyan)}
#connection{border:1px solid var(--line);border-radius:999px;padding:6px 10px;color:var(--muted)}#connection.live{color:var(--cyan);border-color:#277d77}#connection.offline{color:var(--red);border-color:#753145}
.transport{display:grid;grid-template-columns:minmax(170px,1fr) repeat(4,minmax(70px,auto));gap:8px;margin-bottom:12px}.tile,.panel{background:linear-gradient(150deg,#151d2a,#0d121b);border:1px solid var(--line);border-radius:10px;box-shadow:0 12px 35px #0005}.tile{padding:12px}.label{display:block;color:var(--muted);font-size:11px;letter-spacing:.12em;text-transform:uppercase}.value{display:block;margin-top:5px;font-size:clamp(16px,2.5vw,25px);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.controls{display:flex;gap:8px;margin-bottom:12px;overflow:auto;padding-bottom:2px}button{appearance:none;border:1px solid var(--line);border-radius:8px;background:#172133;color:var(--text);font:inherit;font-weight:700;padding:9px 13px;cursor:pointer;white-space:nowrap}button:hover{border-color:var(--cyan)}button:active{transform:translateY(1px)}button.primary{background:#153c3a;border-color:#277d77;color:#a8fff7}button.active{background:#3e305d;border-color:var(--violet)}button.danger{color:#ff9bb0}
.grid{display:grid;grid-template-columns:minmax(0,2.1fr) minmax(240px,.9fr);gap:12px}.panel{position:relative;overflow:hidden}.panel h2{font-size:12px;letter-spacing:.14em;text-transform:uppercase;color:var(--muted);margin:0;padding:11px 13px;border-bottom:1px solid var(--line)}canvas{display:block;width:100%;height:100%}#visual{height:min(64vh,680px);min-height:380px}.side{display:grid;grid-template-rows:minmax(180px,.75fr) minmax(220px,1.25fr);gap:12px}.meter-wrap{padding:13px;height:calc(100% - 38px)}#meters{min-height:150px}.tracks{padding:8px;overflow:auto;height:calc(100% - 38px)}.track{display:grid;grid-template-columns:minmax(0,1fr) auto auto;gap:6px;align-items:center;padding:7px 5px;border-bottom:1px solid #202a3a}.track-name{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.track-note{color:var(--cyan);font-size:12px}.track button{padding:5px 7px;font-size:11px}.activity{height:2px;background:var(--cyan);margin-top:4px;transform-origin:left;transition:transform 80ms linear}
footer{color:var(--muted);font-size:11px;margin-top:10px;display:flex;justify-content:space-between;gap:10px}#error{color:var(--red)}
@media(max-width:850px){.transport{grid-template-columns:repeat(2,minmax(0,1fr))}.transport .song{grid-column:1/-1}.grid{grid-template-columns:1fr}.side{grid-template-columns:1fr 1fr;grid-template-rows:minmax(220px,auto)}#visual{height:52vh}}
@media(max-width:560px){body{padding:8px}.side{grid-template-columns:1fr;grid-template-rows:220px 280px}.transport{grid-template-columns:1fr 1fr}.tile{padding:9px}.brand{font-size:21px}}
</style>
</head>
<body>
<header><div class="brand"><span>trk</span> companion</div><div id="connection">connecting</div></header>
<section class="transport">
  <div class="tile song"><span class="label">Song</span><span class="value" id="song">—</span></div>
  <div class="tile"><span class="label">Transport</span><span class="value" id="playing">STOP</span></div>
  <div class="tile"><span class="label">Pattern</span><span class="value" id="pattern">—</span></div>
  <div class="tile"><span class="label">Row / Tick</span><span class="value" id="position">—</span></div>
  <div class="tile"><span class="label">BPM / LPB</span><span class="value" id="tempo">—</span></div>
</section>
<nav class="controls"><button class="primary" id="play">Play / Pause</button><button class="danger" id="stop">Stop</button><span id="patterns"></span></nav>
<main class="grid">
  <section class="panel"><h2>Pattern piano roll · arrangement</h2><canvas id="visual"></canvas></section>
  <div class="side">
    <section class="panel"><h2>Master response</h2><div class="meter-wrap"><canvas id="meters"></canvas></div></section>
    <section class="panel"><h2>Tracks</h2><div class="tracks" id="tracks"></div></section>
  </div>
</main>
<footer><span>Local loopback session · no project or sample paths exposed</span><span id="error"></span></footer>
<script>
"use strict";
const $=id=>document.getElementById(id), visual=$("visual"), meters=$("meters");
let state=null,lastRevision=-1,inflight=null,lastSeen=0,trackEnergy=[];
const clamp=(v,a=0,b=1)=>Math.max(a,Math.min(b,Number.isFinite(v)?v:0));
function resize(canvas){const r=canvas.getBoundingClientRect(),d=Math.min(devicePixelRatio||1,2);const w=Math.max(1,Math.round(r.width*d)),h=Math.max(1,Math.round(r.height*d));if(canvas.width!==w||canvas.height!==h){canvas.width=w;canvas.height=h}const c=canvas.getContext("2d");c.setTransform(d,0,0,d,0,0);return {c,w:r.width,h:r.height}}
function noteName(p){const names=["C","C♯","D","D♯","E","F","F♯","G","G♯","A","A♯","B"];return names[p%12]+(Math.floor(p/12)-1)}
function setConnection(ok,message){const el=$("connection");el.textContent=message;el.className=ok?"live":"offline"}
function syncDom(){if(!state)return;$("song").textContent=state.songTitle;$("playing").textContent=state.transport.playing?"PLAY":"STOP";$("playing").style.color=state.transport.playing?"var(--cyan)":"var(--muted)";const p=state.activePattern;$("pattern").textContent=p?`${p.index+1} · ${p.name}`:"—";$("position").textContent=`${state.transport.currentRow} / ${state.transport.currentTick}`;$("tempo").textContent=`${state.transport.bpm} / ${state.transport.linesPerBeat}`;renderPatternButtons();renderTracks()}
function renderPatternButtons(){const root=$("patterns"),frag=document.createDocumentFragment();for(const p of state.patterns){const b=document.createElement("button");b.textContent=`${p.index+1} ${p.name}`;if(p.index===state.transport.patternIndex)b.className="active";b.addEventListener("click",()=>action({type:"selectPattern",index:p.index}));frag.appendChild(b)}root.replaceChildren(frag)}
function renderTracks(){const root=$("tracks"),frag=document.createDocumentFragment();state.tracks.forEach((t,i)=>{trackEnergy[i]=Math.max(t.activity||0,(trackEnergy[i]||0)*.82);const row=document.createElement("div");row.className="track";const info=document.createElement("div"),name=document.createElement("div"),note=document.createElement("span"),bar=document.createElement("div");name.className="track-name";name.textContent=`${String(i+1).padStart(2,"0")} ${t.name}`;note.className="track-note";note.textContent=t.activeNote?noteName(t.activeNote.pitch):"";name.append(" ",note);bar.className="activity";bar.style.transform=`scaleX(${clamp(trackEnergy[i])})`;info.append(name,bar);const mute=document.createElement("button"),solo=document.createElement("button");mute.textContent="M";solo.textContent="S";if(t.muted)mute.className="active";if(t.solo)solo.className="active";mute.addEventListener("click",()=>action({type:"toggleTrackMute",index:i}));solo.addEventListener("click",()=>action({type:"toggleTrackSolo",index:i}));row.append(info,mute,solo);frag.appendChild(row)});root.replaceChildren(frag)}
async function action(payload){try{const r=await fetch("/api/action",{method:"POST",headers:{"Content-Type":"application/json","X-Trk-Request":"1"},body:JSON.stringify(payload)});if(!r.ok)throw new Error(`action ${r.status}`);$("error").textContent=""}catch(e){$("error").textContent=e.message}}
async function poll(){if(inflight)inflight.abort();inflight=new AbortController();const timer=setTimeout(()=>inflight.abort(),220);try{const r=await fetch("/api/state",{cache:"no-store",signal:inflight.signal});if(!r.ok)throw new Error(`state ${r.status}`);const next=await r.json();if(next.version!==1)throw new Error("unsupported state version");state=next;lastSeen=performance.now();setConnection(true,"live · 20 Hz");if(next.revision!==lastRevision){lastRevision=next.revision;syncDom()}$("error").textContent=""}catch(e){setConnection(false,"disconnected");$("error").textContent=e.name==="AbortError"?"state timeout":e.message}finally{clearTimeout(timer);inflight=null;setTimeout(poll,50)}}
function rounded(c,x,y,w,h,r){c.beginPath();c.roundRect(x,y,w,h,r);c.fill()}
function drawVisual(now){const {c,w,h}=resize(visual);c.clearRect(0,0,w,h);const g=c.createLinearGradient(0,0,w,h);g.addColorStop(0,"#111a28");g.addColorStop(1,"#080b10");c.fillStyle=g;c.fillRect(0,0,w,h);if(!state||!state.activePattern){c.fillStyle="#8190a8";c.fillText("Waiting for pattern state…",20,30);return}const p=state.activePattern,arrH=Math.max(66,h*.16),rollTop=arrH+16,rollH=h-rollTop-26,rows=Math.max(1,p.rows),tracks=Math.max(1,state.tracks.length),rowW=w/rows;drawArrangement(c,w,arrH);c.strokeStyle="#263247";c.lineWidth=1;for(let r=0;r<=rows;r+=Math.max(1,Math.ceil(rows/16))){const x=r*rowW;c.beginPath();c.moveTo(x,rollTop);c.lineTo(x,h);c.stroke()}for(let t=0;t<=tracks;t++){const y=rollTop+t*rollH/tracks;c.beginPath();c.moveTo(0,y);c.lineTo(w,y);c.stroke()}const pulse=.76+.24*Math.sin(now/180);for(const n of p.notes){const x=n.row*rowW,y=rollTop+n.track*rollH/tracks+3,nh=Math.max(3,rollH/tracks-6),nw=Math.max(2,rowW*.82);c.fillStyle=n.row===state.transport.currentRow?`rgba(86,224,212,${pulse})`:`rgba(167,139,250,${.34+.55*n.velocity/127})`;rounded(c,x+1,y,nw,nh,Math.min(3,nh/2))}const playX=(state.transport.currentRow+.5)*rowW;c.strokeStyle="#ffc857";c.lineWidth=2;c.beginPath();c.moveTo(playX,rollTop);c.lineTo(playX,h);c.stroke();c.fillStyle="#8190a8";c.font="11px ui-monospace,monospace";c.fillText(`${p.name} · ${rows} rows`,10,h-8)}
function drawArrangement(c,w,h){const seq=state.sequence,total=Math.max(1,seq.reduce((s,v)=>s+Math.max(1,v.rows),0)),gap=3;let x=0;c.font="10px ui-monospace,monospace";for(const slot of seq){const sw=Math.max(4,w*Math.max(1,slot.rows)/total),active=slot.active;c.fillStyle=active?"#265d59":"#182234";rounded(c,x+gap/2,8,Math.max(1,sw-gap),h-22,5);if(sw>38){c.fillStyle=active?"#b9fff8":"#8190a8";c.fillText(slot.name,x+6,28,sw-10)}x+=sw}const px=state.transport.currentTick/total*w;c.fillStyle="#ffc857";c.fillRect(clamp(px,0,w-2),4,2,h-14)}
function drawMeters(now){const {c,w,h}=resize(meters);c.clearRect(0,0,w,h);c.fillStyle="#0b1018";c.fillRect(0,0,w,h);if(!state)return;const vals=[state.meters.low,state.meters.mid,state.meters.high,state.meters.rms,state.meters.peak].map(clamp),labels=["LOW","MID","HIGH","RMS","PEAK"],gap=8,bw=(w-gap*6)/5;c.font="10px ui-monospace,monospace";vals.forEach((v,i)=>{const x=gap+i*(bw+gap),bh=(h-28)*v;c.fillStyle="#182234";rounded(c,x,4,bw,h-24,4);const hue=i<3?`hsl(${175+i*32} 70% 60%)`:i===3?"#ffc857":"#ff5e7d";c.fillStyle=hue;rounded(c,x,h-20-bh,bw,bh,4);c.fillStyle="#8190a8";c.textAlign="center";c.fillText(labels[i],x+bw/2,h-5)});c.textAlign="left";const wave=vals[0]*.25+vals[1]*.45+vals[2]*.3;c.strokeStyle="rgba(86,224,212,.38)";c.lineWidth=1.5;c.beginPath();for(let x=0;x<w;x++){const y=h*.45+Math.sin(x*.055+now*.006)*wave*h*.16+Math.sin(x*.017-now*.003)*wave*h*.09;x?c.lineTo(x,y):c.moveTo(x,y)}c.stroke()}
function frame(now){if(lastSeen&&now-lastSeen>500)setConnection(false,"stale");drawVisual(now);drawMeters(now);requestAnimationFrame(frame)}
$("play").addEventListener("click",()=>action({type:"togglePlayback"}));$("stop").addEventListener("click",()=>action({type:"stop"}));
poll();requestAnimationFrame(frame);
</script>
</body>
</html>
"##;
