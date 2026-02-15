/*! For license information please see 2378.746cb068.js.LICENSE.txt */
"use strict";(globalThis.webpackChunkgoose=globalThis.webpackChunkgoose||[]).push([[2378],{27293:(e,t,i)=>{i.d(t,{A:()=>_});var o=i(96540),n=i(74848);function r(e){const{mdxAdmonitionTitle:t,rest:i}=function(e){const t=o.Children.toArray(e),i=t.find(e=>o.isValidElement(e)&&"mdxAdmonitionTitle"===e.type),r=t.filter(e=>e!==i),a=i?.props.children;return{mdxAdmonitionTitle:a,rest:r.length>0?(0,n.jsx)(n.Fragment,{children:r}):null}}(e.children),r=e.title??t;return{...e,...r&&{title:r},children:i}}var a=i(34164),s=i(21312),l=i(17559);const c="admonition_xJq3",d="admonitionHeading_Gvgb",m="admonitionIcon_Rf37",u="admonitionContent_BuS1";function p({type:e,className:t,children:i}){return(0,n.jsx)("div",{className:(0,a.A)(l.G.common.admonition,l.G.common.admonitionType(e),c,t),children:i})}function f({icon:e,title:t}){return(0,n.jsxs)("div",{className:d,children:[(0,n.jsx)("span",{className:m,children:e}),t]})}function h({children:e}){return e?(0,n.jsx)("div",{className:u,children:e}):null}function g(e){const{type:t,icon:i,title:o,children:r,className:a}=e;return(0,n.jsxs)(p,{type:t,className:a,children:[o||i?(0,n.jsx)(f,{title:o,icon:i}):null,(0,n.jsx)(h,{children:r})]})}function x(e){return(0,n.jsx)("svg",{viewBox:"0 0 14 16",...e,children:(0,n.jsx)("path",{fillRule:"evenodd",d:"M6.3 5.69a.942.942 0 0 1-.28-.7c0-.28.09-.52.28-.7.19-.18.42-.28.7-.28.28 0 .52.09.7.28.18.19.28.42.28.7 0 .28-.09.52-.28.7a1 1 0 0 1-.7.3c-.28 0-.52-.11-.7-.3zM8 7.99c-.02-.25-.11-.48-.31-.69-.2-.19-.42-.3-.69-.31H6c-.27.02-.48.13-.69.31-.2.2-.3.44-.31.69h1v3c.02.27.11.5.31.69.2.2.42.31.69.31h1c.27 0 .48-.11.69-.31.2-.19.3-.42.31-.69H8V7.98v.01zM7 2.3c-3.14 0-5.7 2.54-5.7 5.68 0 3.14 2.56 5.7 5.7 5.7s5.7-2.55 5.7-5.7c0-3.15-2.56-5.69-5.7-5.69v.01zM7 .98c3.86 0 7 3.14 7 7s-3.14 7-7 7-7-3.12-7-7 3.14-7 7-7z"})})}const y={icon:(0,n.jsx)(x,{}),title:(0,n.jsx)(s.A,{id:"theme.admonition.note",description:"The default label used for the Note admonition (:::note)",children:"note"})};function b(e){return(0,n.jsx)(g,{...y,...e,className:(0,a.A)("alert alert--secondary",e.className),children:e.children})}function v(e){return(0,n.jsx)("svg",{viewBox:"0 0 12 16",...e,children:(0,n.jsx)("path",{fillRule:"evenodd",d:"M6.5 0C3.48 0 1 2.19 1 5c0 .92.55 2.25 1 3 1.34 2.25 1.78 2.78 2 4v1h5v-1c.22-1.22.66-1.75 2-4 .45-.75 1-2.08 1-3 0-2.81-2.48-5-5.5-5zm3.64 7.48c-.25.44-.47.8-.67 1.11-.86 1.41-1.25 2.06-1.45 3.23-.02.05-.02.11-.02.17H5c0-.06 0-.13-.02-.17-.2-1.17-.59-1.83-1.45-3.23-.2-.31-.42-.67-.67-1.11C2.44 6.78 2 5.65 2 5c0-2.2 2.02-4 4.5-4 1.22 0 2.36.42 3.22 1.19C10.55 2.94 11 3.94 11 5c0 .66-.44 1.78-.86 2.48zM4 14h5c-.23 1.14-1.3 2-2.5 2s-2.27-.86-2.5-2z"})})}const w={icon:(0,n.jsx)(v,{}),title:(0,n.jsx)(s.A,{id:"theme.admonition.tip",description:"The default label used for the Tip admonition (:::tip)",children:"tip"})};function j(e){return(0,n.jsx)(g,{...w,...e,className:(0,a.A)("alert alert--success",e.className),children:e.children})}function N(e){return(0,n.jsx)("svg",{viewBox:"0 0 14 16",...e,children:(0,n.jsx)("path",{fillRule:"evenodd",d:"M7 2.3c3.14 0 5.7 2.56 5.7 5.7s-2.56 5.7-5.7 5.7A5.71 5.71 0 0 1 1.3 8c0-3.14 2.56-5.7 5.7-5.7zM7 1C3.14 1 0 4.14 0 8s3.14 7 7 7 7-3.14 7-7-3.14-7-7-7zm1 3H6v5h2V4zm0 6H6v2h2v-2z"})})}const A={icon:(0,n.jsx)(N,{}),title:(0,n.jsx)(s.A,{id:"theme.admonition.info",description:"The default label used for the Info admonition (:::info)",children:"info"})};function k(e){return(0,n.jsx)(g,{...A,...e,className:(0,a.A)("alert alert--info",e.className),children:e.children})}function z(e){return(0,n.jsx)("svg",{viewBox:"0 0 16 16",...e,children:(0,n.jsx)("path",{fillRule:"evenodd",d:"M8.893 1.5c-.183-.31-.52-.5-.887-.5s-.703.19-.886.5L.138 13.499a.98.98 0 0 0 0 1.001c.193.31.53.501.886.501h13.964c.367 0 .704-.19.877-.5a1.03 1.03 0 0 0 .01-1.002L8.893 1.5zm.133 11.497H6.987v-2.003h2.039v2.003zm0-3.004H6.987V5.987h2.039v4.006z"})})}const $={icon:(0,n.jsx)(z,{}),title:(0,n.jsx)(s.A,{id:"theme.admonition.warning",description:"The default label used for the Warning admonition (:::warning)",children:"warning"})};function C(e){return(0,n.jsx)("svg",{viewBox:"0 0 12 16",...e,children:(0,n.jsx)("path",{fillRule:"evenodd",d:"M5.05.31c.81 2.17.41 3.38-.52 4.31C3.55 5.67 1.98 6.45.9 7.98c-1.45 2.05-1.7 6.53 3.53 7.7-2.2-1.16-2.67-4.52-.3-6.61-.61 2.03.53 3.33 1.94 2.86 1.39-.47 2.3.53 2.27 1.67-.02.78-.31 1.44-1.13 1.81 3.42-.59 4.78-3.42 4.78-5.56 0-2.84-2.53-3.22-1.25-5.61-1.52.13-2.03 1.13-1.89 2.75.09 1.08-1.02 1.8-1.86 1.33-.67-.41-.66-1.19-.06-1.78C8.18 5.31 8.68 2.45 5.05.32L5.03.3l.02.01z"})})}const E={icon:(0,n.jsx)(C,{}),title:(0,n.jsx)(s.A,{id:"theme.admonition.danger",description:"The default label used for the Danger admonition (:::danger)",children:"danger"})};const T={icon:(0,n.jsx)(z,{}),title:(0,n.jsx)(s.A,{id:"theme.admonition.caution",description:"The default label used for the Caution admonition (:::caution)",children:"caution"})};const M={...{note:b,tip:j,info:k,warning:function(e){return(0,n.jsx)(g,{...$,...e,className:(0,a.A)("alert alert--warning",e.className),children:e.children})},danger:function(e){return(0,n.jsx)(g,{...E,...e,className:(0,a.A)("alert alert--danger",e.className),children:e.children})}},...{secondary:e=>(0,n.jsx)(b,{title:"secondary",...e}),important:e=>(0,n.jsx)(k,{title:"important",...e}),success:e=>(0,n.jsx)(j,{title:"success",...e}),caution:function(e){return(0,n.jsx)(g,{...T,...e,className:(0,a.A)("alert alert--warning",e.className),children:e.children})}}};function _(e){const t=r(e),i=(o=t.type,M[o]||(console.warn(`No admonition component found for admonition type "${o}". Using Info as fallback.`),M.info));var o;return(0,n.jsx)(i,{...t})}},62636:(e,t,i)=>{i.d(t,{Ay:()=>K});var o=i(96540);let n={data:""},r=e=>{if("object"==typeof window){let t=(e?e.querySelector("#_goober"):window._goober)||Object.assign(document.createElement("style"),{innerHTML:" ",id:"_goober"});return t.nonce=window.__nonce__,t.parentNode||(e||document.head).appendChild(t),t.firstChild}return e||n},a=/(?:([\u0080-\uFFFF\w-%@]+) *:? *([^{;]+?);|([^;}{]*?) *{)|(}\s*)/g,s=/\/\*[^]*?\*\/|  +/g,l=/\n+/g,c=(e,t)=>{let i="",o="",n="";for(let r in e){let a=e[r];"@"==r[0]?"i"==r[1]?i=r+" "+a+";":o+="f"==r[1]?c(a,r):r+"{"+c(a,"k"==r[1]?"":t)+"}":"object"==typeof a?o+=c(a,t?t.replace(/([^,])+/g,e=>r.replace(/([^,]*:\S+\([^)]*\))|([^,])+/g,t=>/&/.test(t)?t.replace(/&/g,e):e?e+" "+t:t)):r):null!=a&&(r=/^--/.test(r)?r:r.replace(/[A-Z]/g,"-$&").toLowerCase(),n+=c.p?c.p(r,a):r+":"+a+";")}return i+(t&&n?t+"{"+n+"}":n)+o},d={},m=e=>{if("object"==typeof e){let t="";for(let i in e)t+=i+m(e[i]);return t}return e},u=(e,t,i,o,n)=>{let r=m(e),u=d[r]||(d[r]=(e=>{let t=0,i=11;for(;t<e.length;)i=101*i+e.charCodeAt(t++)>>>0;return"go"+i})(r));if(!d[u]){let t=r!==e?e:(e=>{let t,i,o=[{}];for(;t=a.exec(e.replace(s,""));)t[4]?o.shift():t[3]?(i=t[3].replace(l," ").trim(),o.unshift(o[0][i]=o[0][i]||{})):o[0][t[1]]=t[2].replace(l," ").trim();return o[0]})(e);d[u]=c(n?{["@keyframes "+u]:t}:t,i?"":"."+u)}let p=i&&d.g?d.g:null;return i&&(d.g=d[u]),((e,t,i,o)=>{o?t.data=t.data.replace(o,e):-1===t.data.indexOf(e)&&(t.data=i?e+t.data:t.data+e)})(d[u],t,o,p),u};function p(e){let t=this||{},i=e.call?e(t.p):e;return u(i.unshift?i.raw?((e,t,i)=>e.reduce((e,o,n)=>{let r=t[n];if(r&&r.call){let e=r(i),t=e&&e.props&&e.props.className||/^go/.test(e)&&e;r=t?"."+t:e&&"object"==typeof e?e.props?"":c(e,""):!1===e?"":e}return e+o+(null==r?"":r)},""))(i,[].slice.call(arguments,1),t.p):i.reduce((e,i)=>Object.assign(e,i&&i.call?i(t.p):i),{}):i,r(t.target),t.g,t.o,t.k)}p.bind({g:1});let f,h,g,x=p.bind({k:1});function y(e,t){let i=this||{};return function(){let o=arguments;function n(r,a){let s=Object.assign({},r),l=s.className||n.className;i.p=Object.assign({theme:h&&h()},s),i.o=/ *go\d+/.test(l),s.className=p.apply(i,o)+(l?" "+l:""),t&&(s.ref=a);let c=e;return e[0]&&(c=s.as||e,delete s.as),g&&c[0]&&g(s),f(c,s)}return t?t(n):n}}var b=(e,t)=>(e=>"function"==typeof e)(e)?e(t):e,v=(()=>{let e=0;return()=>(++e).toString()})(),w=(()=>{let e;return()=>{if(void 0===e&&typeof window<"u"){let t=matchMedia("(prefers-reduced-motion: reduce)");e=!t||t.matches}return e}})(),j="default",N=(e,t)=>{let{toastLimit:i}=e.settings;switch(t.type){case 0:return{...e,toasts:[t.toast,...e.toasts].slice(0,i)};case 1:return{...e,toasts:e.toasts.map(e=>e.id===t.toast.id?{...e,...t.toast}:e)};case 2:let{toast:o}=t;return N(e,{type:e.toasts.find(e=>e.id===o.id)?1:0,toast:o});case 3:let{toastId:n}=t;return{...e,toasts:e.toasts.map(e=>e.id===n||void 0===n?{...e,dismissed:!0,visible:!1}:e)};case 4:return void 0===t.toastId?{...e,toasts:[]}:{...e,toasts:e.toasts.filter(e=>e.id!==t.toastId)};case 5:return{...e,pausedAt:t.time};case 6:let r=t.time-(e.pausedAt||0);return{...e,pausedAt:void 0,toasts:e.toasts.map(e=>({...e,pauseDuration:e.pauseDuration+r}))}}},A=[],k={toasts:[],pausedAt:void 0,settings:{toastLimit:20}},z={},$=(e,t=j)=>{z[t]=N(z[t]||k,e),A.forEach(([e,i])=>{e===t&&i(z[t])})},C=e=>Object.keys(z).forEach(t=>$(e,t)),E=(e=j)=>t=>{$(t,e)},T=e=>(t,i)=>{let o=((e,t="blank",i)=>({createdAt:Date.now(),visible:!0,dismissed:!1,type:t,ariaProps:{role:"status","aria-live":"polite"},message:e,pauseDuration:0,...i,id:(null==i?void 0:i.id)||v()}))(t,e,i);return E(o.toasterId||(e=>Object.keys(z).find(t=>z[t].toasts.some(t=>t.id===e)))(o.id))({type:2,toast:o}),o.id},M=(e,t)=>T("blank")(e,t);M.error=T("error"),M.success=T("success"),M.loading=T("loading"),M.custom=T("custom"),M.dismiss=(e,t)=>{let i={type:3,toastId:e};t?E(t)(i):C(i)},M.dismissAll=e=>M.dismiss(void 0,e),M.remove=(e,t)=>{let i={type:4,toastId:e};t?E(t)(i):C(i)},M.removeAll=e=>M.remove(void 0,e),M.promise=(e,t,i)=>{let o=M.loading(t.loading,{...i,...null==i?void 0:i.loading});return"function"==typeof e&&(e=e()),e.then(e=>{let n=t.success?b(t.success,e):void 0;return n?M.success(n,{id:o,...i,...null==i?void 0:i.success}):M.dismiss(o),e}).catch(e=>{let n=t.error?b(t.error,e):void 0;n?M.error(n,{id:o,...i,...null==i?void 0:i.error}):M.dismiss(o)}),e};var _=x`
from {
  transform: scale(0) rotate(45deg);
	opacity: 0;
}
to {
 transform: scale(1) rotate(45deg);
  opacity: 1;
}`,L=x`
from {
  transform: scale(0);
  opacity: 0;
}
to {
  transform: scale(1);
  opacity: 1;
}`,H=x`
from {
  transform: scale(0) rotate(90deg);
	opacity: 0;
}
to {
  transform: scale(1) rotate(90deg);
	opacity: 1;
}`,I=y("div")`
  width: 20px;
  opacity: 0;
  height: 20px;
  border-radius: 10px;
  background: ${e=>e.primary||"#ff4b4b"};
  position: relative;
  transform: rotate(45deg);

  animation: ${_} 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275)
    forwards;
  animation-delay: 100ms;

  &:after,
  &:before {
    content: '';
    animation: ${L} 0.15s ease-out forwards;
    animation-delay: 150ms;
    position: absolute;
    border-radius: 3px;
    opacity: 0;
    background: ${e=>e.secondary||"#fff"};
    bottom: 9px;
    left: 4px;
    height: 2px;
    width: 12px;
  }

  &:before {
    animation: ${H} 0.15s ease-out forwards;
    animation-delay: 180ms;
    transform: rotate(90deg);
  }
`,B=x`
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
`,O=y("div")`
  width: 12px;
  height: 12px;
  box-sizing: border-box;
  border: 2px solid;
  border-radius: 100%;
  border-color: ${e=>e.secondary||"#e0e0e0"};
  border-right-color: ${e=>e.primary||"#616161"};
  animation: ${B} 1s linear infinite;
`,R=x`
from {
  transform: scale(0) rotate(45deg);
	opacity: 0;
}
to {
  transform: scale(1) rotate(45deg);
	opacity: 1;
}`,F=x`
0% {
	height: 0;
	width: 0;
	opacity: 0;
}
40% {
  height: 0;
	width: 6px;
	opacity: 1;
}
100% {
  opacity: 1;
  height: 10px;
}`,D=y("div")`
  width: 20px;
  opacity: 0;
  height: 20px;
  border-radius: 10px;
  background: ${e=>e.primary||"#61d345"};
  position: relative;
  transform: rotate(45deg);

  animation: ${R} 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275)
    forwards;
  animation-delay: 100ms;
  &:after {
    content: '';
    box-sizing: border-box;
    animation: ${F} 0.2s ease-out forwards;
    opacity: 0;
    animation-delay: 200ms;
    position: absolute;
    border-right: 2px solid;
    border-bottom: 2px solid;
    border-color: ${e=>e.secondary||"#fff"};
    bottom: 6px;
    left: 6px;
    height: 10px;
    width: 6px;
  }
`,S=y("div")`
  position: absolute;
`,W=y("div")`
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
  min-width: 20px;
  min-height: 20px;
`,V=x`
from {
  transform: scale(0.6);
  opacity: 0.4;
}
to {
  transform: scale(1);
  opacity: 1;
}`,G=y("div")`
  position: relative;
  transform: scale(0.6);
  opacity: 0.4;
  min-width: 20px;
  animation: ${V} 0.3s 0.12s cubic-bezier(0.175, 0.885, 0.32, 1.275)
    forwards;
`,q=({toast:e})=>{let{icon:t,type:i,iconTheme:n}=e;return void 0!==t?"string"==typeof t?o.createElement(G,null,t):t:"blank"===i?null:o.createElement(W,null,o.createElement(O,{...n}),"loading"!==i&&o.createElement(S,null,"error"===i?o.createElement(I,{...n}):o.createElement(D,{...n})))},P=e=>`\n0% {transform: translate3d(0,${-200*e}%,0) scale(.6); opacity:.5;}\n100% {transform: translate3d(0,0,0) scale(1); opacity:1;}\n`,Z=e=>`\n0% {transform: translate3d(0,0,-1px) scale(1); opacity:1;}\n100% {transform: translate3d(0,${-150*e}%,-1px) scale(.6); opacity:0;}\n`,J=y("div")`
  display: flex;
  align-items: center;
  background: #fff;
  color: #363636;
  line-height: 1.3;
  will-change: transform;
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.1), 0 3px 3px rgba(0, 0, 0, 0.05);
  max-width: 350px;
  pointer-events: auto;
  padding: 8px 10px;
  border-radius: 8px;
`,U=y("div")`
  display: flex;
  justify-content: center;
  margin: 4px 10px;
  color: inherit;
  flex: 1 1 auto;
  white-space: pre-line;
`;o.memo(({toast:e,position:t,style:i,children:n})=>{let r=e.height?((e,t)=>{let i=e.includes("top")?1:-1,[o,n]=w()?["0%{opacity:0;} 100%{opacity:1;}","0%{opacity:1;} 100%{opacity:0;}"]:[P(i),Z(i)];return{animation:t?`${x(o)} 0.35s cubic-bezier(.21,1.02,.73,1) forwards`:`${x(n)} 0.4s forwards cubic-bezier(.06,.71,.55,1)`}})(e.position||t||"top-center",e.visible):{opacity:0},a=o.createElement(q,{toast:e}),s=o.createElement(U,{...e.ariaProps},b(e.message,e));return o.createElement(J,{className:e.className,style:{...r,...i,...e.style}},"function"==typeof n?n({icon:a,message:s}):o.createElement(o.Fragment,null,a,s))});!function(e,t,i,o){c.p=t,f=e,h=i,g=o}(o.createElement);p`
  z-index: 9999;
  > * {
    pointer-events: auto;
  }
`;var K=M},75395:(e,t,i)=>{i.d(t,{A:()=>s});var o=i(96540);const n=(...e)=>e.filter((e,t,i)=>Boolean(e)&&""!==e.trim()&&i.indexOf(e)===t).join(" ").trim();var r={xmlns:"http://www.w3.org/2000/svg",width:24,height:24,viewBox:"0 0 24 24",fill:"none",stroke:"currentColor",strokeWidth:2,strokeLinecap:"round",strokeLinejoin:"round"};const a=(0,o.forwardRef)(({color:e="currentColor",size:t=24,strokeWidth:i=2,absoluteStrokeWidth:a,className:s="",children:l,iconNode:c,...d},m)=>(0,o.createElement)("svg",{ref:m,...r,width:t,height:t,stroke:e,strokeWidth:a?24*Number(i)/Number(t):i,className:n("lucide",s),...d},[...c.map(([e,t])=>(0,o.createElement)(e,t)),...Array.isArray(l)?l:[l]])),s=(e,t)=>{const i=(0,o.forwardRef)(({className:i,...r},s)=>{return(0,o.createElement)(a,{ref:s,iconNode:t,className:n(`lucide-${l=e,l.replace(/([a-z0-9])([A-Z])/g,"$1-$2").toLowerCase()}`,i),...r});var l});return i.displayName=`${e}`,i}}}]);