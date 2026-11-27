const settingsView=(()=>{

const $=id=>document.getElementById(id), root=$('tab-settings'), area=$('settings-cards'), cards=[...area.querySelectorAll(':scope > .card-glass')];
area.firstElementChild.hidden=true;
const paths={account:'M20 21a8 8 0 0 0-16 0 M16 8a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z',sound:'M4 10v4m4-8v12m4-15v18m4-14v10m4-7v4',discord:'M4 4h16v12H9l-5 4Z M8 9h8',look:'M4 6h16M4 12h16M4 18h16 M8 3v6m8 0v6m-6 0v6'};
for(const button of root.querySelectorAll('nav button'))button.insertAdjacentHTML('afterbegin','<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="'+paths[button.dataset.group]+'"/></svg>');
const tokenBlock=$('settings-token-input').parentElement.parentElement;
const disclosure=document.createElement('details');
const summary=document.createElement('summary');summary.textContent='Вход по токену';
tokenBlock.replaceWith(disclosure);disclosure.append(summary,tokenBlock);tokenBlock.className='';
const groups=['account','sound','discord','sound','look'];
function show(group){cards.forEach((card,i)=>card.hidden=groups[i]!==group);root.querySelectorAll('nav button').forEach(b=>b.setAttribute('aria-pressed',b.dataset.group===group));$('settings-notice').textContent=''}
root.querySelectorAll('nav button').forEach(b=>b.onclick=()=>show(b.dataset.group));show('account');
const palettes={
 wine:['#170f15','#32202b'],oled:['#000000','#141414'],light:['#ece9ea','#faf8f9'],
 flame:['#1a100c','#33221a'],cyan:['#10141c','#1d2c37'],emerald:['#0e1815','#1c332a'],
 amethyst:['#18121d','#33263e'],sunset:['#201417','#3b2629'],midnight:['#101420','#222f49']
};
function select(selector,btn){document.querySelectorAll(selector).forEach(b=>b.setAttribute('aria-pressed',b===btn))}
const themeSamples={
 wine:['#682438','#ad5368'],oled:['#161616','#373737'],light:['#fff','#c9cdd2'],
 flame:['#9d3b17','#ff773c'],cyan:['#187987','#a481bf'],emerald:['#185447','#55a688'],
 amethyst:['#543968','#aa7ac7'],sunset:['#934550','#ed975f'],midnight:['#293b60','#647da4']
};
const themeKey='onyx.settings.custom.v1';
const picker=$('theme-color');
let seed='#687294';
try{
 const saved=JSON.parse(localStorage.getItem(themeKey));
 const previous=saved?.seed ?? saved?.panel;
 if(/^#[0-9a-f]{6}$/i.test(previous))seed=previous;
}catch{}
picker.value=seed;
function mixThemeColor(hex,neutral,weight){
 const channels=hex.slice(1).match(/../g);
 return '#'+channels.map(channel=>Math.round(parseInt(channel,16)*weight+neutral*(1-weight)).toString(16).padStart(2,'0')).join('');
}
function luminance(hex){
 const [r,g,b]=hex.slice(1).match(/../g).map(pair=>{
  const v=parseInt(pair,16)/255;
  return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;
 });
 return .2126*r+.7152*g+.0722*b;
}
function updateCustomSample(){
 const color=picker.value;
 const rgb=color.slice(1).match(/../g).map(pair=>parseInt(pair,16));
 const chroma=(Math.max(...rgb)-Math.min(...rgb))/255;
 const gray=Math.round(.2126*rgb[0]+.7152*rgb[1]+.0722*rgb[2]);
 let base=mixThemeColor(color,gray,1-.65*chroma);
 const light=contrast('#000000',base)>=contrast('#ffffff',base);
 // Leave enough contrast for secondary labels on adjacent surfaces.
 const ink=light?'#000000':'#ffffff';
 if(contrast(ink,base)<6) base=mixThemeColor(base,light?255:0,.78);
 const panel=mixThemeColor(base,luminance(base)<.025?255:0,light ? .92 : .82);
 const highlight=mixThemeColor(color,255,.75);
 palettes.custom=[base,panel];
 themeSamples.custom=[panel,highlight];
 const sample=document.querySelector('.theme-select-btn[data-theme="custom"] div');
 sample.style.setProperty('--sample-base',base);
 sample.style.setProperty('--sample-mid',panel);
 sample.style.setProperty('--sample-tip',highlight);
}
updateCustomSample();
picker.oninput=()=>{
 updateCustomSample();window.applyTheme('custom',false,document.documentElement.style.getPropertyValue('--accent'));
 try{localStorage.setItem(themeKey,JSON.stringify({seed:picker.value}));}
 catch{$('settings-notice').textContent='Сохранение недоступно в этом браузере';}
};
function bindColorMenu(menu,plane,valueControl,code,initial,onChange){
 const picker={value:initial,oninput:()=>onChange(picker.value)};
let hue=0,saturation=0,brightness=0;
function drawPicker(){

 menu.style.setProperty('--seed',picker.value);
 menu.style.setProperty('--hue',hue);
 menu.style.setProperty('--sat',hue/360*100+'%');
 menu.style.setProperty('--shade',1-brightness);
 menu.style.setProperty('--bright-color','hsl('+hue+' '+(saturation?100:0)+'% '+(100-50*saturation)+'%)');
 menu.style.setProperty('--top',(1-saturation)*100+'%');
 valueControl.value=Math.round(brightness*100);
 code.value=picker.value.toUpperCase();
}
function readColor(hex){
 const [r,g,b]=hex.slice(1).match(/../g).map(c=>parseInt(c,16)/255);
 brightness=Math.max(r,g,b);
 const span=brightness-Math.min(r,g,b);
 saturation=brightness?span/brightness:0;
 if(span){
  const sector=brightness===r?(g-b)/span:brightness===g?(b-r)/span+2:(r-g)/span+4;
  hue=(sector*60+360)%360;
 }
 picker.value=hex.toLowerCase();drawPicker();
}
function applyPicker(){
 const channel=n=>{
  const k=(n+hue/60)%6;
  const v=brightness*(1-saturation*Math.max(0,Math.min(k,4-k,1)));
  return Math.round(255*v).toString(16).padStart(2,'0');
 };
 picker.value='#'+[5,3,1].map(channel).join('');drawPicker();picker.oninput();
}
function pickPoint(e){
 const rect=plane.getBoundingClientRect();
 hue=359.99*Math.min(1,Math.max(0,(e.clientX-rect.left)/rect.width));
 saturation=1-Math.min(1,Math.max(0,(e.clientY-rect.top)/rect.height));
 applyPicker();
}
plane.onpointerdown=e=>{if(e.button!==0)return;plane.setPointerCapture(e.pointerId);pickPoint(e)};
plane.onpointermove=e=>{if(plane.hasPointerCapture(e.pointerId))pickPoint(e)};
valueControl.oninput=()=>{brightness=Number(valueControl.value)/100;applyPicker()};
code.oninput=()=>{if(/^#?[0-9a-f]{6}$/i.test(code.value)){readColor('#'+code.value.replace('#',''));picker.oninput()}};
code.onblur=()=>drawPicker();
menu.onkeydown=e=>{if(e.key==='Escape'){menu.open=false;menu.querySelector('summary').focus()}};
readColor(initial);
 return readColor;
}
bindColorMenu($('color-menu'),$('color-plane'),$('color-value'),$('color-code'),seed,color=>{
 picker.value=color;picker.oninput();
});
const syncAccentMenu=bindColorMenu($('accent-menu'),$('accent-plane'),$('accent-value'),$('custom-accent-hex'),'#ffffff',color=>window.applyAccent(color,'Свой цвет'));
for(const button of document.querySelectorAll('.theme-select-btn')){
 const theme=button.dataset.theme;
 const sample=button.firstElementChild;
 sample.setAttribute('aria-hidden','true');
 sample.style.setProperty('--sample-base',palettes[theme][0]);
 sample.style.setProperty('--sample-mid',themeSamples[theme][0]);
 sample.style.setProperty('--sample-tip',themeSamples[theme][1]);
}
function contrast(a,b){
 const x=luminance(a),y=luminance(b);
 return (Math.max(x,y)+.05)/(Math.min(x,y)+.05);
}
function updateCustomAccent(accent){
 const [base,panel]=palettes.custom;
 const ink=contrast('#000000',base)>=contrast('#ffffff',base)?0:255;
 let mark=accent;
 for(let step=1;step<=20 && Math.min(contrast(mark,panel),contrast(mark,base))<4.5;step++){
  mark=mixThemeColor(accent,ink,1-step/20);
 }
 document.documentElement.style.setProperty('--custom-mark',mark);
 return contrast('#000000',accent)>=contrast('#ffffff',accent)?'#000000':'#ffffff';
}
function applyTheme(theme){
 const [base,panel]=palettes[theme];
 const light=contrast('#000000',base)>=contrast('#ffffff',base);
 const neutral=light?0:255;
 const ink=light?'#000000':'#ffffff';
 const surface=mixThemeColor(panel,neutral,.945);
 // Keep secondary text readable even on the raised input and button surfaces.
 let muted=ink;
 for(const weight of [.34,.24,.14,0]){
  const candidate=mixThemeColor(panel,neutral,weight);
  if(Math.min(contrast(candidate,panel),contrast(candidate,surface))>=4.5){muted=candidate;break}
 }
 const tokens={base,panel,ink,muted,surface,'outer-ink':ink,
  line:'color-mix(in srgb,var(--ink) 20%,transparent)'};
 const login=$('login-modal');
 for(const [name,value] of Object.entries(tokens)){
  root.style.setProperty('--'+name,value);
  login.style.setProperty('--'+name,value);
 }
 login.style.colorScheme=light?'light':'dark';
 if(theme==='custom'){
  for(const [name,value] of Object.entries(tokens))document.documentElement.style.setProperty('--custom-'+name,value);
 }
 root.dataset.theme=theme;
 root.style.colorScheme=light?'light':'dark';
 $('custom-theme-editor').hidden=theme!=='custom';
 select('.theme-select-btn',document.querySelector('.theme-select-btn[data-theme="'+theme+'"]'));
}

const blur=$('bg-blur-slider');
function renderRange(input){
 input.style.setProperty('--fill',100*(input.value-input.min)/(input.max-input.min)+'%');
}
function setBlur(value){
 blur.value=value;
 document.documentElement.style.setProperty('--settings-wall-blur',blur.value+'px');
 $('bg-blur-val').textContent=blur.value+' px';
 renderRange(blur);
}
setBlur(localStorage.getItem('onyx.wallpaper.blur')||0);
blur.addEventListener('input',()=>{setBlur(blur.value);localStorage.setItem('onyx.wallpaper.blur',blur.value)});
$('btn-reset-customization').addEventListener('click',()=>{setBlur(0);localStorage.removeItem('onyx.wallpaper.blur')});
root.addEventListener('input',event=>{if(event.target.type==='range')renderRange(event.target)});
root.querySelectorAll('input[type=range]').forEach(renderRange);
const storedTheme=localStorage.getItem('sc_desktop_theme');
applyTheme(Object.hasOwn(palettes,storedTheme)?storedTheme:'oled');
const storedAccent=localStorage.getItem('sc_desktop_accent') || document.documentElement.style.getPropertyValue('--accent').trim() || '#f43f5e';
if(/^#[0-9a-f]{6}$/i.test(storedAccent)){
 syncAccentMenu(storedAccent);
 const selected=area.querySelector('.accent-btn[data-color="'+storedAccent+'"]');
 select('.accent-btn',selected);
 $('current-accent-label').textContent=storedAccent+(selected?' ('+selected.dataset.name+')':'');
}
return {applyTheme,updateCustomAccent,syncAccentMenu,palettes,renderRange,customIsLight:()=>contrast('#000000',palettes.custom[0])>=contrast('#ffffff',palettes.custom[0])};
})();
