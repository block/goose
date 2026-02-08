"use strict";(globalThis.webpackChunkgoose=globalThis.webpackChunkgoose||[]).push([[6400],{27293:(e,t,i)=>{i.d(t,{A:()=>_});var n=i(96540),o=i(74848);function a(e){const{mdxAdmonitionTitle:t,rest:i}=function(e){const t=n.Children.toArray(e),i=t.find(e=>n.isValidElement(e)&&"mdxAdmonitionTitle"===e.type),a=t.filter(e=>e!==i),r=i?.props.children;return{mdxAdmonitionTitle:r,rest:a.length>0?(0,o.jsx)(o.Fragment,{children:a}):null}}(e.children),a=e.title??t;return{...e,...a&&{title:a},children:i}}var r=i(34164),s=i(21312),l=i(17559);const c="admonition_xJq3",d="admonitionHeading_Gvgb",m="admonitionIcon_Rf37",u="admonitionContent_BuS1";function p({type:e,className:t,children:i}){return(0,o.jsx)("div",{className:(0,r.A)(l.G.common.admonition,l.G.common.admonitionType(e),c,t),children:i})}function f({icon:e,title:t}){return(0,o.jsxs)("div",{className:d,children:[(0,o.jsx)("span",{className:m,children:e}),t]})}function h({children:e}){return e?(0,o.jsx)("div",{className:u,children:e}):null}function g(e){const{type:t,icon:i,title:n,children:a,className:r}=e;return(0,o.jsxs)(p,{type:t,className:r,children:[n||i?(0,o.jsx)(f,{title:n,icon:i}):null,(0,o.jsx)(h,{children:a})]})}function x(e){return(0,o.jsx)("svg",{viewBox:"0 0 14 16",...e,children:(0,o.jsx)("path",{fillRule:"evenodd",d:"M6.3 5.69a.942.942 0 0 1-.28-.7c0-.28.09-.52.28-.7.19-.18.42-.28.7-.28.28 0 .52.09.7.28.18.19.28.42.28.7 0 .28-.09.52-.28.7a1 1 0 0 1-.7.3c-.28 0-.52-.11-.7-.3zM8 7.99c-.02-.25-.11-.48-.31-.69-.2-.19-.42-.3-.69-.31H6c-.27.02-.48.13-.69.31-.2.2-.3.44-.31.69h1v3c.02.27.11.5.31.69.2.2.42.31.69.31h1c.27 0 .48-.11.69-.31.2-.19.3-.42.31-.69H8V7.98v.01zM7 2.3c-3.14 0-5.7 2.54-5.7 5.68 0 3.14 2.56 5.7 5.7 5.7s5.7-2.55 5.7-5.7c0-3.15-2.56-5.69-5.7-5.69v.01zM7 .98c3.86 0 7 3.14 7 7s-3.14 7-7 7-7-3.12-7-7 3.14-7 7-7z"})})}const y={icon:(0,o.jsx)(x,{}),title:(0,o.jsx)(s.A,{id:"theme.admonition.note",description:"The default label used for the Note admonition (:::note)",children:"note"})};function b(e){return(0,o.jsx)(g,{...y,...e,className:(0,r.A)("alert alert--secondary",e.className),children:e.children})}function v(e){return(0,o.jsx)("svg",{viewBox:"0 0 12 16",...e,children:(0,o.jsx)("path",{fillRule:"evenodd",d:"M6.5 0C3.48 0 1 2.19 1 5c0 .92.55 2.25 1 3 1.34 2.25 1.78 2.78 2 4v1h5v-1c.22-1.22.66-1.75 2-4 .45-.75 1-2.08 1-3 0-2.81-2.48-5-5.5-5zm3.64 7.48c-.25.44-.47.8-.67 1.11-.86 1.41-1.25 2.06-1.45 3.23-.02.05-.02.11-.02.17H5c0-.06 0-.13-.02-.17-.2-1.17-.59-1.83-1.45-3.23-.2-.31-.42-.67-.67-1.11C2.44 6.78 2 5.65 2 5c0-2.2 2.02-4 4.5-4 1.22 0 2.36.42 3.22 1.19C10.55 2.94 11 3.94 11 5c0 .66-.44 1.78-.86 2.48zM4 14h5c-.23 1.14-1.3 2-2.5 2s-2.27-.86-2.5-2z"})})}const j={icon:(0,o.jsx)(v,{}),title:(0,o.jsx)(s.A,{id:"theme.admonition.tip",description:"The default label used for the Tip admonition (:::tip)",children:"tip"})};function w(e){return(0,o.jsx)(g,{...j,...e,className:(0,r.A)("alert alert--success",e.className),children:e.children})}function A(e){return(0,o.jsx)("svg",{viewBox:"0 0 14 16",...e,children:(0,o.jsx)("path",{fillRule:"evenodd",d:"M7 2.3c3.14 0 5.7 2.56 5.7 5.7s-2.56 5.7-5.7 5.7A5.71 5.71 0 0 1 1.3 8c0-3.14 2.56-5.7 5.7-5.7zM7 1C3.14 1 0 4.14 0 8s3.14 7 7 7 7-3.14 7-7-3.14-7-7-7zm1 3H6v5h2V4zm0 6H6v2h2v-2z"})})}const N={icon:(0,o.jsx)(A,{}),title:(0,o.jsx)(s.A,{id:"theme.admonition.info",description:"The default label used for the Info admonition (:::info)",children:"info"})};function z(e){return(0,o.jsx)(g,{...N,...e,className:(0,r.A)("alert alert--info",e.className),children:e.children})}function $(e){return(0,o.jsx)("svg",{viewBox:"0 0 16 16",...e,children:(0,o.jsx)("path",{fillRule:"evenodd",d:"M8.893 1.5c-.183-.31-.52-.5-.887-.5s-.703.19-.886.5L.138 13.499a.98.98 0 0 0 0 1.001c.193.31.53.501.886.501h13.964c.367 0 .704-.19.877-.5a1.03 1.03 0 0 0 .01-1.002L8.893 1.5zm.133 11.497H6.987v-2.003h2.039v2.003zm0-3.004H6.987V5.987h2.039v4.006z"})})}const k={icon:(0,o.jsx)($,{}),title:(0,o.jsx)(s.A,{id:"theme.admonition.warning",description:"The default label used for the Warning admonition (:::warning)",children:"warning"})};function C(e){return(0,o.jsx)("svg",{viewBox:"0 0 12 16",...e,children:(0,o.jsx)("path",{fillRule:"evenodd",d:"M5.05.31c.81 2.17.41 3.38-.52 4.31C3.55 5.67 1.98 6.45.9 7.98c-1.45 2.05-1.7 6.53 3.53 7.7-2.2-1.16-2.67-4.52-.3-6.61-.61 2.03.53 3.33 1.94 2.86 1.39-.47 2.3.53 2.27 1.67-.02.78-.31 1.44-1.13 1.81 3.42-.59 4.78-3.42 4.78-5.56 0-2.84-2.53-3.22-1.25-5.61-1.52.13-2.03 1.13-1.89 2.75.09 1.08-1.02 1.8-1.86 1.33-.67-.41-.66-1.19-.06-1.78C8.18 5.31 8.68 2.45 5.05.32L5.03.3l.02.01z"})})}const E={icon:(0,o.jsx)(C,{}),title:(0,o.jsx)(s.A,{id:"theme.admonition.danger",description:"The default label used for the Danger admonition (:::danger)",children:"danger"})};const T={icon:(0,o.jsx)($,{}),title:(0,o.jsx)(s.A,{id:"theme.admonition.caution",description:"The default label used for the Caution admonition (:::caution)",children:"caution"})};const M={...{note:b,tip:w,info:z,warning:function(e){return(0,o.jsx)(g,{...k,...e,className:(0,r.A)("alert alert--warning",e.className),children:e.children})},danger:function(e){return(0,o.jsx)(g,{...E,...e,className:(0,r.A)("alert alert--danger",e.className),children:e.children})}},...{secondary:e=>(0,o.jsx)(b,{title:"secondary",...e}),important:e=>(0,o.jsx)(z,{title:"important",...e}),success:e=>(0,o.jsx)(w,{title:"success",...e}),caution:function(e){return(0,o.jsx)(g,{...T,...e,className:(0,r.A)("alert alert--warning",e.className),children:e.children})}}};function _(e){const t=a(e),i=(n=t.type,M[n]||(console.warn(`No admonition component found for admonition type "${n}". Using Info as fallback.`),M.info));var n;return(0,o.jsx)(i,{...t})}},62636:(e,t,i)=>{i.d(t,{Ay:()=>K});var n=i(96540);let o={data:""},a=e=>{if("object"==typeof window){let t=(e?e.querySelector("#_goober"):window._goober)||Object.assign(document.createElement("style"),{innerHTML:" ",id:"_goober"});return t.nonce=window.__nonce__,t.parentNode||(e||document.head).appendChild(t),t.firstChild}return e||o},r=/(?:([\u0080-\uFFFF\w-%@]+) *:? *([^{;]+?);|([^;}{]*?) *{)|(}\s*)/g,s=/\/\*[^]*?\*\/|  +/g,l=/\n+/g,c=(e,t)=>{let i="",n="",o="";for(let a in e){let r=e[a];"@"==a[0]?"i"==a[1]?i=a+" "+r+";":n+="f"==a[1]?c(r,a):a+"{"+c(r,"k"==a[1]?"":t)+"}":"object"==typeof r?n+=c(r,t?t.replace(/([^,])+/g,e=>a.replace(/([^,]*:\S+\([^)]*\))|([^,])+/g,t=>/&/.test(t)?t.replace(/&/g,e):e?e+" "+t:t)):a):null!=r&&(a=/^--/.test(a)?a:a.replace(/[A-Z]/g,"-$&").toLowerCase(),o+=c.p?c.p(a,r):a+":"+r+";")}return i+(t&&o?t+"{"+o+"}":o)+n},d={},m=e=>{if("object"==typeof e){let t="";for(let i in e)t+=i+m(e[i]);return t}return e},u=(e,t,i,n,o)=>{let a=m(e),u=d[a]||(d[a]=(e=>{let t=0,i=11;for(;t<e.length;)i=101*i+e.charCodeAt(t++)>>>0;return"go"+i})(a));if(!d[u]){let t=a!==e?e:(e=>{let t,i,n=[{}];for(;t=r.exec(e.replace(s,""));)t[4]?n.shift():t[3]?(i=t[3].replace(l," ").trim(),n.unshift(n[0][i]=n[0][i]||{})):n[0][t[1]]=t[2].replace(l," ").trim();return n[0]})(e);d[u]=c(o?{["@keyframes "+u]:t}:t,i?"":"."+u)}let p=i&&d.g?d.g:null;return i&&(d.g=d[u]),((e,t,i,n)=>{n?t.data=t.data.replace(n,e):-1===t.data.indexOf(e)&&(t.data=i?e+t.data:t.data+e)})(d[u],t,n,p),u};function p(e){let t=this||{},i=e.call?e(t.p):e;return u(i.unshift?i.raw?((e,t,i)=>e.reduce((e,n,o)=>{let a=t[o];if(a&&a.call){let e=a(i),t=e&&e.props&&e.props.className||/^go/.test(e)&&e;a=t?"."+t:e&&"object"==typeof e?e.props?"":c(e,""):!1===e?"":e}return e+n+(null==a?"":a)},""))(i,[].slice.call(arguments,1),t.p):i.reduce((e,i)=>Object.assign(e,i&&i.call?i(t.p):i),{}):i,a(t.target),t.g,t.o,t.k)}p.bind({g:1});let f,h,g,x=p.bind({k:1});function y(e,t){let i=this||{};return function(){let n=arguments;function o(a,r){let s=Object.assign({},a),l=s.className||o.className;i.p=Object.assign({theme:h&&h()},s),i.o=/ *go\d+/.test(l),s.className=p.apply(i,n)+(l?" "+l:""),t&&(s.ref=r);let c=e;return e[0]&&(c=s.as||e,delete s.as),g&&c[0]&&g(s),f(c,s)}return t?t(o):o}}var b=(e,t)=>(e=>"function"==typeof e)(e)?e(t):e,v=(()=>{let e=0;return()=>(++e).toString()})(),j=(()=>{let e;return()=>{if(void 0===e&&typeof window<"u"){let t=matchMedia("(prefers-reduced-motion: reduce)");e=!t||t.matches}return e}})(),w="default",A=(e,t)=>{let{toastLimit:i}=e.settings;switch(t.type){case 0:return{...e,toasts:[t.toast,...e.toasts].slice(0,i)};case 1:return{...e,toasts:e.toasts.map(e=>e.id===t.toast.id?{...e,...t.toast}:e)};case 2:let{toast:n}=t;return A(e,{type:e.toasts.find(e=>e.id===n.id)?1:0,toast:n});case 3:let{toastId:o}=t;return{...e,toasts:e.toasts.map(e=>e.id===o||void 0===o?{...e,dismissed:!0,visible:!1}:e)};case 4:return void 0===t.toastId?{...e,toasts:[]}:{...e,toasts:e.toasts.filter(e=>e.id!==t.toastId)};case 5:return{...e,pausedAt:t.time};case 6:let a=t.time-(e.pausedAt||0);return{...e,pausedAt:void 0,toasts:e.toasts.map(e=>({...e,pauseDuration:e.pauseDuration+a}))}}},N=[],z={toasts:[],pausedAt:void 0,settings:{toastLimit:20}},$={},k=(e,t=w)=>{$[t]=A($[t]||z,e),N.forEach(([e,i])=>{e===t&&i($[t])})},C=e=>Object.keys($).forEach(t=>k(e,t)),E=(e=w)=>t=>{k(t,e)},T=e=>(t,i)=>{let n=((e,t="blank",i)=>({createdAt:Date.now(),visible:!0,dismissed:!1,type:t,ariaProps:{role:"status","aria-live":"polite"},message:e,pauseDuration:0,...i,id:(null==i?void 0:i.id)||v()}))(t,e,i);return E(n.toasterId||(e=>Object.keys($).find(t=>$[t].toasts.some(t=>t.id===e)))(n.id))({type:2,toast:n}),n.id},M=(e,t)=>T("blank")(e,t);M.error=T("error"),M.success=T("success"),M.loading=T("loading"),M.custom=T("custom"),M.dismiss=(e,t)=>{let i={type:3,toastId:e};t?E(t)(i):C(i)},M.dismissAll=e=>M.dismiss(void 0,e),M.remove=(e,t)=>{let i={type:4,toastId:e};t?E(t)(i):C(i)},M.removeAll=e=>M.remove(void 0,e),M.promise=(e,t,i)=>{let n=M.loading(t.loading,{...i,...null==i?void 0:i.loading});return"function"==typeof e&&(e=e()),e.then(e=>{let o=t.success?b(t.success,e):void 0;return o?M.success(o,{id:n,...i,...null==i?void 0:i.success}):M.dismiss(n),e}).catch(e=>{let o=t.error?b(t.error,e):void 0;o?M.error(o,{id:n,...i,...null==i?void 0:i.error}):M.dismiss(n)}),e};var _=x`
from {
  transform: scale(0) rotate(45deg);
	opacity: 0;
}
to {
 transform: scale(1) rotate(45deg);
  opacity: 1;
}`,H=x`
from {
  transform: scale(0);
  opacity: 0;
}
to {
  transform: scale(1);
  opacity: 1;
}`,I=x`
from {
  transform: scale(0) rotate(90deg);
	opacity: 0;
}
to {
  transform: scale(1) rotate(90deg);
	opacity: 1;
}`,L=y("div")`
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
    animation: ${H} 0.15s ease-out forwards;
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
    animation: ${I} 0.15s ease-out forwards;
    animation-delay: 180ms;
    transform: rotate(90deg);
  }
`,O=x`
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
`,B=y("div")`
  width: 12px;
  height: 12px;
  box-sizing: border-box;
  border: 2px solid;
  border-radius: 100%;
  border-color: ${e=>e.secondary||"#e0e0e0"};
  border-right-color: ${e=>e.primary||"#616161"};
  animation: ${O} 1s linear infinite;
`,F=x`
from {
  transform: scale(0) rotate(45deg);
	opacity: 0;
}
to {
  transform: scale(1) rotate(45deg);
	opacity: 1;
}`,R=x`
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

  animation: ${F} 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275)
    forwards;
  animation-delay: 100ms;
  &:after {
    content: '';
    box-sizing: border-box;
    animation: ${R} 0.2s ease-out forwards;
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
`,V=y("div")`
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
  min-width: 20px;
  min-height: 20px;
`,G=x`
from {
  transform: scale(0.6);
  opacity: 0.4;
}
to {
  transform: scale(1);
  opacity: 1;
}`,q=y("div")`
  position: relative;
  transform: scale(0.6);
  opacity: 0.4;
  min-width: 20px;
  animation: ${G} 0.3s 0.12s cubic-bezier(0.175, 0.885, 0.32, 1.275)
    forwards;
`,P=({toast:e})=>{let{icon:t,type:i,iconTheme:o}=e;return void 0!==t?"string"==typeof t?n.createElement(q,null,t):t:"blank"===i?null:n.createElement(V,null,n.createElement(B,{...o}),"loading"!==i&&n.createElement(S,null,"error"===i?n.createElement(L,{...o}):n.createElement(D,{...o})))},J=e=>`\n0% {transform: translate3d(0,${-200*e}%,0) scale(.6); opacity:.5;}\n100% {transform: translate3d(0,0,0) scale(1); opacity:1;}\n`,U=e=>`\n0% {transform: translate3d(0,0,-1px) scale(1); opacity:1;}\n100% {transform: translate3d(0,${-150*e}%,-1px) scale(.6); opacity:0;}\n`,W=y("div")`
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
`,Z=y("div")`
  display: flex;
  justify-content: center;
  margin: 4px 10px;
  color: inherit;
  flex: 1 1 auto;
  white-space: pre-line;
`;n.memo(({toast:e,position:t,style:i,children:o})=>{let a=e.height?((e,t)=>{let i=e.includes("top")?1:-1,[n,o]=j()?["0%{opacity:0;} 100%{opacity:1;}","0%{opacity:1;} 100%{opacity:0;}"]:[J(i),U(i)];return{animation:t?`${x(n)} 0.35s cubic-bezier(.21,1.02,.73,1) forwards`:`${x(o)} 0.4s forwards cubic-bezier(.06,.71,.55,1)`}})(e.position||t||"top-center",e.visible):{opacity:0},r=n.createElement(P,{toast:e}),s=n.createElement(Z,{...e.ariaProps},b(e.message,e));return n.createElement(W,{className:e.className,style:{...a,...i,...e.style}},"function"==typeof o?o({icon:r,message:s}):n.createElement(n.Fragment,null,r,s))});!function(e,t,i,n){c.p=t,f=e,h=i,g=n}(n.createElement);p`
  z-index: 9999;
  > * {
    pointer-events: auto;
  }
`;var K=M}}]);