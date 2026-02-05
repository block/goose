/**
 * @license React
 * react-reconciler-reflection.development.js
 *
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

if (process.env.NODE_ENV !== "production") {
  (function() {
'use strict';

var React = require('react');

var ReactSharedInternals = React.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED;

function error(format) {
  {
    {
      for (var _len2 = arguments.length, args = new Array(_len2 > 1 ? _len2 - 1 : 0), _key2 = 1; _key2 < _len2; _key2++) {
        args[_key2 - 1] = arguments[_key2];
      }

      printWarning('error', format, args);
    }
  }
}

function printWarning(level, format, args) {
  // When changing this logic, you might want to also
  // update consoleWithStackDev.www.js as well.
  {
    var ReactDebugCurrentFrame = ReactSharedInternals.ReactDebugCurrentFrame;
    var stack = ReactDebugCurrentFrame.getStackAddendum();

    if (stack !== '') {
      format += '%s';
      args = args.concat([stack]);
    } // eslint-disable-next-line react-internal/safe-string-coercion


    var argsWithFormat = args.map(function (item) {
      return String(item);
    }); // Careful: RN currently depends on this prefix

    argsWithFormat.unshift('Warning: ' + format); // We intentionally don't use spread (or .apply) directly because it
    // breaks IE9: https://github.com/facebook/react/issues/13610
    // eslint-disable-next-line react-internal/no-production-logging

    Function.prototype.apply.call(console[level], console, argsWithFormat);
  }
}

/**
 * `ReactInstanceMap` maintains a mapping from a public facing stateful
 * instance (key) and the internal representation (value). This allows public
 * methods to accept the user facing instance as an argument and map them back
 * to internal methods.
 *
 * Note that this module is currently shared and assumed to be stateless.
 * If this becomes an actual Map, that will break.
 */
function get(key) {
  return key._reactInternals;
}

var FunctionComponent = 0;
var ClassComponent = 1;
var IndeterminateComponent = 2; // Before we know whether it is function or class

var HostRoot = 3; // Root of a host tree. Could be nested inside another node.

var HostPortal = 4; // A subtree. Could be an entry point to a different renderer.

var HostComponent = 5;
var HostText = 6;
var Fragment = 7;
var Mode = 8;
var ContextConsumer = 9;
var ContextProvider = 10;
var ForwardRef = 11;
var Profiler = 12;
var SuspenseComponent = 13;
var MemoComponent = 14;
var SimpleMemoComponent = 15;
var LazyComponent = 16;
var IncompleteClassComponent = 17;
var DehydratedFragment = 18;
var SuspenseListComponent = 19;
var ScopeComponent = 21;
var OffscreenComponent = 22;
var CacheComponent = 24;
var TracingMarkerComponent = 25;

// ATTENTION
var REACT_PORTAL_TYPE = Symbol.for('react.portal');
var REACT_FRAGMENT_TYPE = Symbol.for('react.fragment');
var REACT_STRICT_MODE_TYPE = Symbol.for('react.strict_mode');
var REACT_PROFILER_TYPE = Symbol.for('react.profiler');
var REACT_PROVIDER_TYPE = Symbol.for('react.provider');
var REACT_CONTEXT_TYPE = Symbol.for('react.context');
var REACT_FORWARD_REF_TYPE = Symbol.for('react.forward_ref');
var REACT_SUSPENSE_TYPE = Symbol.for('react.suspense');
var REACT_SUSPENSE_LIST_TYPE = Symbol.for('react.suspense_list');
var REACT_MEMO_TYPE = Symbol.for('react.memo');
var REACT_LAZY_TYPE = Symbol.for('react.lazy');

function getWrappedName(outerType, innerType, wrapperName) {
  var displayName = outerType.displayName;

  if (displayName) {
    return displayName;
  }

  var functionName = innerType.displayName || innerType.name || '';
  return functionName !== '' ? wrapperName + "(" + functionName + ")" : wrapperName;
} // Keep in sync with react-reconciler/getComponentNameFromFiber


function getContextName(type) {
  return type.displayName || 'Context';
} // Note that the reconciler package should generally prefer to use getComponentNameFromFiber() instead.


function getComponentNameFromType(type) {
  if (type == null) {
    // Host root, text node or just invalid type.
    return null;
  }

  {
    if (typeof type.tag === 'number') {
      error('Received an unexpected object in getComponentNameFromType(). ' + 'This is likely a bug in React. Please file an issue.');
    }
  }

  if (typeof type === 'function') {
    return type.displayName || type.name || null;
  }

  if (typeof type === 'string') {
    return type;
  }

  switch (type) {
    case REACT_FRAGMENT_TYPE:
      return 'Fragment';

    case REACT_PORTAL_TYPE:
      return 'Portal';

    case REACT_PROFILER_TYPE:
      return 'Profiler';

    case REACT_STRICT_MODE_TYPE:
      return 'StrictMode';

    case REACT_SUSPENSE_TYPE:
      return 'Suspense';

    case REACT_SUSPENSE_LIST_TYPE:
      return 'SuspenseList';

  }

  if (typeof type === 'object') {
    switch (type.$$typeof) {
      case REACT_CONTEXT_TYPE:
        var context = type;
        return getContextName(context) + '.Consumer';

      case REACT_PROVIDER_TYPE:
        var provider = type;
        return getContextName(provider._context) + '.Provider';

      case REACT_FORWARD_REF_TYPE:
        return getWrappedName(type, type.render, 'ForwardRef');

      case REACT_MEMO_TYPE:
        var outerName = type.displayName || null;

        if (outerName !== null) {
          return outerName;
        }

        return getComponentNameFromType(type.type) || 'Memo';

      case REACT_LAZY_TYPE:
        {
          var lazyComponent = type;
          var payload = lazyComponent._payload;
          var init = lazyComponent._init;

          try {
            return getComponentNameFromType(init(payload));
          } catch (x) {
            return null;
          }
        }

      // eslint-disable-next-line no-fallthrough
    }
  }

  return null;
}

function getWrappedName$1(outerType, innerType, wrapperName) {
  var functionName = innerType.displayName || innerType.name || '';
  return outerType.displayName || (functionName !== '' ? wrapperName + "(" + functionName + ")" : wrapperName);
} // Keep in sync with shared/getComponentNameFromType


function getContextName$1(type) {
  return type.displayName || 'Context';
}

function getComponentNameFromFiber(fiber) {
  var tag = fiber.tag,
      type = fiber.type;

  switch (tag) {
    case CacheComponent:
      return 'Cache';

    case ContextConsumer:
      var context = type;
      return getContextName$1(context) + '.Consumer';

    case ContextProvider:
      var provider = type;
      return getContextName$1(provider._context) + '.Provider';

    case DehydratedFragment:
      return 'DehydratedFragment';

    case ForwardRef:
      return getWrappedName$1(type, type.render, 'ForwardRef');

    case Fragment:
      return 'Fragment';

    case HostComponent:
      // Host component type is the display name (e.g. "div", "View")
      return type;

    case HostPortal:
      return 'Portal';

    case HostRoot:
      return 'Root';

    case HostText:
      return 'Text';

    case LazyComponent:
      // Name comes from the type in this case; we don't have a tag.
      return getComponentNameFromType(type);

    case Mode:
      if (type === REACT_STRICT_MODE_TYPE) {
        // Don't be less specific than shared/getComponentNameFromType
        return 'StrictMode';
      }

      return 'Mode';

    case OffscreenComponent:
      return 'Offscreen';

    case Profiler:
      return 'Profiler';

    case ScopeComponent:
      return 'Scope';

    case SuspenseComponent:
      return 'Suspense';

    case SuspenseListComponent:
      return 'SuspenseList';

    case TracingMarkerComponent:
      return 'TracingMarker';
    // The display name for this tags come from the user-provided type:

    case ClassComponent:
    case FunctionComponent:
    case IncompleteClassComponent:
    case IndeterminateComponent:
    case MemoComponent:
    case SimpleMemoComponent:
      if (typeof type === 'function') {
        return type.displayName || type.name || null;
      }

      if (typeof type === 'string') {
        return type;
      }

      break;

  }

  return null;
}

// Don't change these two values. They're used by React Dev Tools.
var NoFlags =
/*                      */
0;

var Placement =
/*                    */
2;
var Hydrating =
/*                    */
4096;

var ReactCurrentOwner = ReactSharedInternals.ReactCurrentOwner;
function getNearestMountedFiber(fiber) {
  var node = fiber;
  var nearestMounted = fiber;

  if (!fiber.alternate) {
    // If there is no alternate, this might be a new tree that isn't inserted
    // yet. If it is, then it will have a pending insertion effect on it.
    var nextNode = node;

    do {
      node = nextNode;

      if ((node.flags & (Placement | Hydrating)) !== NoFlags) {
        // This is an insertion or in-progress hydration. The nearest possible
        // mounted fiber is the parent but we need to continue to figure out
        // if that one is still mounted.
        nearestMounted = node.return;
      }

      nextNode = node.return;
    } while (nextNode);
  } else {
    while (node.return) {
      node = node.return;
    }
  }

  if (node.tag === HostRoot) {
    // TODO: Check if this was a nested HostRoot when used with
    // renderContainerIntoSubtree.
    return nearestMounted;
  } // If we didn't hit the root, that means that we're in an disconnected tree
  // that has been unmounted.


  return null;
}
function getSuspenseInstanceFromFiber(fiber) {
  if (fiber.tag === SuspenseComponent) {
    var suspenseState = fiber.memoizedState;

    if (suspenseState === null) {
      var current = fiber.alternate;

      if (current !== null) {
        suspenseState = current.memoizedState;
      }
    }

    if (suspenseState !== null) {
      return suspenseState.dehydrated;
    }
  }

  return null;
}
function getContainerFromFiber(fiber) {
  return fiber.tag === HostRoot ? fiber.stateNode.containerInfo : null;
}
function isFiberMounted(fiber) {
  return getNearestMountedFiber(fiber) === fiber;
}
function isMounted(component) {
  {
    var owner = ReactCurrentOwner.current;

    if (owner !== null && owner.tag === ClassComponent) {
      var ownerFiber = owner;
      var instance = ownerFiber.stateNode;

      if (!instance._warnedAboutRefsInRender) {
        error('%s is accessing isMounted inside its render() function. ' + 'render() should be a pure function of props and state. It should ' + 'never access something that requires stale data from the previous ' + 'render, such as refs. Move this logic to componentDidMount and ' + 'componentDidUpdate instead.', getComponentNameFromFiber(ownerFiber) || 'A component');
      }

      instance._warnedAboutRefsInRender = true;
    }
  }

  var fiber = get(component);

  if (!fiber) {
    return false;
  }

  return getNearestMountedFiber(fiber) === fiber;
}

function assertIsMounted(fiber) {
  if (getNearestMountedFiber(fiber) !== fiber) {
    throw new Error('Unable to find node on an unmounted component.');
  }
}

function findCurrentFiberUsingSlowPath(fiber) {
  var alternate = fiber.alternate;

  if (!alternate) {
    // If there is no alternate, then we only need to check if it is mounted.
    var nearestMounted = getNearestMountedFiber(fiber);

    if (nearestMounted === null) {
      throw new Error('Unable to find node on an unmounted component.');
    }

    if (nearestMounted !== fiber) {
      return null;
    }

    return fiber;
  } // If we have two possible branches, we'll walk backwards up to the root
  // to see what path the root points to. On the way we may hit one of the
  // special cases and we'll deal with them.


  var a = fiber;
  var b = alternate;

  while (true) {
    var parentA = a.return;

    if (parentA === null) {
      // We're at the root.
      break;
    }

    var parentB = parentA.alternate;

    if (parentB === null) {
      // There is no alternate. This is an unusual case. Currently, it only
      // happens when a Suspense component is hidden. An extra fragment fiber
      // is inserted in between the Suspense fiber and its children. Skip
      // over this extra fragment fiber and proceed to the next parent.
      var nextParent = parentA.return;

      if (nextParent !== null) {
        a = b = nextParent;
        continue;
      } // If there's no parent, we're at the root.


      break;
    } // If both copies of the parent fiber point to the same child, we can
    // assume that the child is current. This happens when we bailout on low
    // priority: the bailed out fiber's child reuses the current child.


    if (parentA.child === parentB.child) {
      var child = parentA.child;

      while (child) {
        if (child === a) {
          // We've determined that A is the current branch.
          assertIsMounted(parentA);
          return fiber;
        }

        if (child === b) {
          // We've determined that B is the current branch.
          assertIsMounted(parentA);
          return alternate;
        }

        child = child.sibling;
      } // We should never have an alternate for any mounting node. So the only
      // way this could possibly happen is if this was unmounted, if at all.


      throw new Error('Unable to find node on an unmounted component.');
    }

    if (a.return !== b.return) {
      // The return pointer of A and the return pointer of B point to different
      // fibers. We assume that return pointers never criss-cross, so A must
      // belong to the child set of A.return, and B must belong to the child
      // set of B.return.
      a = parentA;
      b = parentB;
    } else {
      // The return pointers point to the same fiber. We'll have to use the
      // default, slow path: scan the child sets of each parent alternate to see
      // which child belongs to which set.
      //
      // Search parent A's child set
      var didFindChild = false;
      var _child = parentA.child;

      while (_child) {
        if (_child === a) {
          didFindChild = true;
          a = parentA;
          b = parentB;
          break;
        }

        if (_child === b) {
          didFindChild = true;
          b = parentA;
          a = parentB;
          break;
        }

        _child = _child.sibling;
      }

      if (!didFindChild) {
        // Search parent B's child set
        _child = parentB.child;

        while (_child) {
          if (_child === a) {
            didFindChild = true;
            a = parentB;
            b = parentA;
            break;
          }

          if (_child === b) {
            didFindChild = true;
            b = parentB;
            a = parentA;
            break;
          }

          _child = _child.sibling;
        }

        if (!didFindChild) {
          throw new Error('Child was not found in either parent set. This indicates a bug ' + 'in React related to the return pointer. Please file an issue.');
        }
      }
    }

    if (a.alternate !== b) {
      throw new Error("Return fibers should always be each others' alternates. " + 'This error is likely caused by a bug in React. Please file an issue.');
    }
  } // If the root is not a host container, we're in a disconnected tree. I.e.
  // unmounted.


  if (a.tag !== HostRoot) {
    throw new Error('Unable to find node on an unmounted component.');
  }

  if (a.stateNode.current === a) {
    // We've determined that A is the current branch.
    return fiber;
  } // Otherwise B has to be current branch.


  return alternate;
}
function findCurrentHostFiber(parent) {
  var currentParent = findCurrentFiberUsingSlowPath(parent);
  return currentParent !== null ? findCurrentHostFiberImpl(currentParent) : null;
}

function findCurrentHostFiberImpl(node) {
  // Next we'll drill down this component to find the first HostComponent/Text.
  if (node.tag === HostComponent || node.tag === HostText) {
    return node;
  }

  var child = node.child;

  while (child !== null) {
    var match = findCurrentHostFiberImpl(child);

    if (match !== null) {
      return match;
    }

    child = child.sibling;
  }

  return null;
}

function findCurrentHostFiberWithNoPortals(parent) {
  var currentParent = findCurrentFiberUsingSlowPath(parent);
  return currentParent !== null ? findCurrentHostFiberWithNoPortalsImpl(currentParent) : null;
}

function findCurrentHostFiberWithNoPortalsImpl(node) {
  // Next we'll drill down this component to find the first HostComponent/Text.
  if (node.tag === HostComponent || node.tag === HostText) {
    return node;
  }

  var child = node.child;

  while (child !== null) {
    if (child.tag !== HostPortal) {
      var match = findCurrentHostFiberWithNoPortalsImpl(child);

      if (match !== null) {
        return match;
      }
    }

    child = child.sibling;
  }

  return null;
}

function isFiberSuspenseAndTimedOut(fiber) {
  var memoizedState = fiber.memoizedState;
  return fiber.tag === SuspenseComponent && memoizedState !== null && memoizedState.dehydrated === null;
}
function doesFiberContain(parentFiber, childFiber) {
  var node = childFiber;
  var parentFiberAlternate = parentFiber.alternate;

  while (node !== null) {
    if (node === parentFiber || node === parentFiberAlternate) {
      return true;
    }

    node = node.return;
  }

  return false;
}

exports.doesFiberContain = doesFiberContain;
exports.findCurrentFiberUsingSlowPath = findCurrentFiberUsingSlowPath;
exports.findCurrentHostFiber = findCurrentHostFiber;
exports.findCurrentHostFiberWithNoPortals = findCurrentHostFiberWithNoPortals;
exports.getContainerFromFiber = getContainerFromFiber;
exports.getNearestMountedFiber = getNearestMountedFiber;
exports.getSuspenseInstanceFromFiber = getSuspenseInstanceFromFiber;
exports.isFiberMounted = isFiberMounted;
exports.isFiberSuspenseAndTimedOut = isFiberSuspenseAndTimedOut;
exports.isMounted = isMounted;
  })();
}
