#!/usr/bin/env node
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { inspectMacosSigningReadiness } = require('../ui/desktop/scripts/lib/macosSigningReadiness.cjs');

const args = new Set(process.argv.slice(2));
const requireSigned = args.has('--require-signed');

const readiness = inspectMacosSigningReadiness(process.env);

console.log(`requested_mode=${readiness.requestedMode}`);
console.log(`signing_requested=${readiness.signingRequested ? 'yes' : 'no'}`);
console.log(`ready_for_signed_release=${readiness.readyForSignedRelease ? 'yes' : 'no'}`);
console.log(
  `missing_secrets=${readiness.signingRequested ? readiness.missingSecrets.join(',') || 'none' : 'not_applicable'}`
);
console.log(
  `invalid_secrets=${readiness.signingRequested ? readiness.invalidSecrets.join(',') || 'none' : 'not_applicable'}`
);
console.log(`fallback=${readiness.fallbackMessage}`);
console.log(`external_conditions=${readiness.externalConditions.join('; ')}`);

if (readiness.signingRequested && readiness.blockerSummary) {
  console.error(`signing_blockers=${readiness.blockerSummary}`);
}

if (requireSigned && !readiness.readyForSignedRelease) {
  console.error(
    'Signed macOS release requested, but signing/notarization preflight is not satisfied.'
  );
  process.exit(1);
}
