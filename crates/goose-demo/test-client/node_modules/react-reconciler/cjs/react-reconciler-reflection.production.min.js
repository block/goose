/**
 * @license React
 * react-reconciler-reflection.production.min.js
 *
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';function h(a){for(var b="https://reactjs.org/docs/error-decoder.html?invariant="+a,c=1;c<arguments.length;c++)b+="&args[]="+encodeURIComponent(arguments[c]);return"Minified React error #"+a+"; visit "+b+" for the full message or use the non-minified dev environment for full errors and additional helpful warnings."}require("react");
function l(a){var b=a,c=a;if(a.alternate)for(;b.return;)b=b.return;else{a=b;do b=a,0!==(b.flags&4098)&&(c=b.return),a=b.return;while(a)}return 3===b.tag?c:null}function m(a){if(l(a)!==a)throw Error(h(188));}
function n(a){var b=a.alternate;if(!b){b=l(a);if(null===b)throw Error(h(188));return b!==a?null:a}for(var c=a,e=b;;){var f=c.return;if(null===f)break;var d=f.alternate;if(null===d){e=f.return;if(null!==e){c=e;continue}break}if(f.child===d.child){for(d=f.child;d;){if(d===c)return m(f),a;if(d===e)return m(f),b;d=d.sibling}throw Error(h(188));}if(c.return!==e.return)c=f,e=d;else{for(var k=!1,g=f.child;g;){if(g===c){k=!0;c=f;e=d;break}if(g===e){k=!0;e=f;c=d;break}g=g.sibling}if(!k){for(g=d.child;g;){if(g===
c){k=!0;c=d;e=f;break}if(g===e){k=!0;e=d;c=f;break}g=g.sibling}if(!k)throw Error(h(189));}}if(c.alternate!==e)throw Error(h(190));}if(3!==c.tag)throw Error(h(188));return c.stateNode.current===c?a:b}function p(a){if(5===a.tag||6===a.tag)return a;for(a=a.child;null!==a;){var b=p(a);if(null!==b)return b;a=a.sibling}return null}function q(a){if(5===a.tag||6===a.tag)return a;for(a=a.child;null!==a;){if(4!==a.tag){var b=q(a);if(null!==b)return b}a=a.sibling}return null}
exports.doesFiberContain=function(a,b){for(var c=a.alternate;null!==b;){if(b===a||b===c)return!0;b=b.return}return!1};exports.findCurrentFiberUsingSlowPath=n;exports.findCurrentHostFiber=function(a){a=n(a);return null!==a?p(a):null};exports.findCurrentHostFiberWithNoPortals=function(a){a=n(a);return null!==a?q(a):null};exports.getContainerFromFiber=function(a){return 3===a.tag?a.stateNode.containerInfo:null};exports.getNearestMountedFiber=l;
exports.getSuspenseInstanceFromFiber=function(a){if(13===a.tag){var b=a.memoizedState;null===b&&(a=a.alternate,null!==a&&(b=a.memoizedState));if(null!==b)return b.dehydrated}return null};exports.isFiberMounted=function(a){return l(a)===a};exports.isFiberSuspenseAndTimedOut=function(a){var b=a.memoizedState;return 13===a.tag&&null!==b&&null===b.dehydrated};exports.isMounted=function(a){return(a=a._reactInternals)?l(a)===a:!1};
