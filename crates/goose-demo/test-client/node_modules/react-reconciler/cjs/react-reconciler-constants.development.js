/**
 * @license React
 * react-reconciler-constants.development.js
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

var SyncLane =
/*                        */
1;
var InputContinuousLane =
/*             */
4;
var DefaultLane =
/*                     */
16;
var IdleLane =
/*                        */
536870912;

var DiscreteEventPriority = SyncLane;
var ContinuousEventPriority = InputContinuousLane;
var DefaultEventPriority = DefaultLane;
var IdleEventPriority = IdleLane;

var LegacyRoot = 0;
var ConcurrentRoot = 1;

exports.ConcurrentRoot = ConcurrentRoot;
exports.ContinuousEventPriority = ContinuousEventPriority;
exports.DefaultEventPriority = DefaultEventPriority;
exports.DiscreteEventPriority = DiscreteEventPriority;
exports.IdleEventPriority = IdleEventPriority;
exports.LegacyRoot = LegacyRoot;
  })();
}
