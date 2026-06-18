#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

function parseArgs(argv) {
  const args = {
    arch: '',
    expectedMode: '',
    evidenceDir: '',
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--arch') {
      args.arch = argv[index + 1] || '';
      index += 1;
      continue;
    }
    if (arg === '--expected-mode') {
      args.expectedMode = argv[index + 1] || '';
      index += 1;
      continue;
    }
    if (arg === '--evidence-dir') {
      args.evidenceDir = argv[index + 1] || '';
      index += 1;
      continue;
    }
    throw new Error(`Unknown argument: ${arg}`);
  }

  if (!args.arch) {
    throw new Error('Missing required argument: --arch');
  }
  if (!args.expectedMode) {
    throw new Error('Missing required argument: --expected-mode');
  }
  if (!args.evidenceDir) {
    throw new Error('Missing required argument: --evidence-dir');
  }

  return args;
}

function readOptionalFile(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }
  return fs.readFileSync(filePath, 'utf8');
}

function parseKeyValueOutput(contents) {
  if (!contents) {
    return {};
  }

  const parsed = {};
  for (const line of contents.split(/\r?\n/)) {
    const separatorIndex = line.indexOf('=');
    if (separatorIndex <= 0) {
      continue;
    }

    const key = line.slice(0, separatorIndex).trim();
    const value = line.slice(separatorIndex + 1).trim();
    parsed[key] = value;
  }
  return parsed;
}

function truncate(value, maxLength = 240) {
  if (!value || value.length <= maxLength) {
    return value || 'not_collected';
  }
  return `${value.slice(0, maxLength - 3)}...`;
}

function buildVerdict(expectedMode, preflight, bundle) {
  if (!preflight.collected) {
    return {
      status: 'incomplete',
      reason: 'signing preflight output was not collected',
    };
  }

  if (expectedMode === 'signed' && preflight.values.ready_for_signed_release !== 'yes') {
    return {
      status: 'blocked',
      reason: 'signed release preflight is not satisfied; this is an Apple secrets or certificate blocker',
    };
  }

  if (!bundle.collected) {
    return {
      status: 'incomplete',
      reason: 'bundle verification output was not collected',
    };
  }

  if (bundle.values.bundle_check !== 'ok') {
    return {
      status: 'blocked',
      reason: 'bundle verification did not complete successfully',
    };
  }

  if (expectedMode === 'signed') {
    const spctlAccepted = /\baccepted\b/i.test(bundle.values.spctl || '');
    const staplerValidated = /The validate action worked/i.test(bundle.values.stapler || '');
    if (!spctlAccepted || !staplerValidated) {
      return {
        status: 'blocked',
        reason: 'signed bundle is missing accepted spctl or stapler validation evidence',
      };
    }
  }

  return {
    status: 'passed',
    reason:
      expectedMode === 'signed'
        ? 'signed/notarized evidence looks complete'
        : 'local-preview evidence looks complete',
  };
}

function renderMarkdown({ arch, expectedMode, preflight, bundle, verdict, generatedAt }) {
  const lines = [
    '# Security Goose macOS Release Evidence',
    '',
    `- generated_at: ${generatedAt}`,
    `- arch: ${arch}`,
    `- expected_mode: ${expectedMode}`,
    `- verdict: ${verdict.status}`,
    `- verdict_reason: ${verdict.reason}`,
    '',
    '## Signing preflight',
    '',
    `- collected: ${preflight.collected ? 'yes' : 'no'}`,
    `- requested_mode: ${preflight.values.requested_mode || 'not_collected'}`,
    `- signing_requested: ${preflight.values.signing_requested || 'not_collected'}`,
    `- ready_for_signed_release: ${preflight.values.ready_for_signed_release || 'not_collected'}`,
    `- missing_secrets: ${preflight.values.missing_secrets || 'not_collected'}`,
    `- invalid_secrets: ${preflight.values.invalid_secrets || 'not_collected'}`,
    `- signing_blockers: ${truncate(preflight.values.signing_blockers)}`,
    `- fallback: ${truncate(preflight.values.fallback)}`,
    `- external_conditions: ${truncate(preflight.values.external_conditions)}`,
  ];

  if (preflight.stderrHint) {
    lines.push(`- blocker_hint: ${truncate(preflight.stderrHint)}`);
  }

  lines.push(
    '',
    '## Bundle verification',
    '',
    `- collected: ${bundle.collected ? 'yes' : 'no'}`,
    `- bundle_check: ${bundle.values.bundle_check || 'not_collected'}`,
    `- bundle: ${bundle.values.bundle || 'not_collected'}`,
    `- zip: ${bundle.values.zip || 'not_collected'}`,
    `- codesign_team: ${bundle.values.codesign_team || 'not_collected'}`,
    `- spctl: ${truncate(bundle.values.spctl)}`,
    `- stapler: ${truncate(bundle.values.stapler)}`,
    ''
  );

  if (bundle.stderrHint) {
    lines.push(`- bundle_error_hint: ${truncate(bundle.stderrHint)}`, '');
  }

  if (expectedMode === 'signed') {
    lines.push('## Signed rehearsal gate', '');
    lines.push('- Required success shape:');
    lines.push('  - ready_for_signed_release=yes');
    lines.push('  - bundle_check=ok');
    lines.push('  - codesign_team is not `not set`');
    lines.push('  - spctl contains `accepted`');
    lines.push('  - stapler contains `The validate action worked.`');
    lines.push('');
    lines.push('- If preflight is blocked, treat it as an Apple secrets / certificate / account-permission blocker.');
    lines.push('- If bundle_check is missing, the build or notarization flow failed before evidence capture finished.');
  } else {
    lines.push('## Local preview gate', '');
    lines.push('- Required success shape:');
    lines.push('  - bundle_check=ok');
    lines.push('  - codesign_team is `not set`');
    lines.push('  - spctl may still be rejected because local-preview is not notarized');
  }

  lines.push('');
  return `${lines.join('\n')}\n`;
}

function collectLastNonKeyValueLine(contents) {
  if (!contents) {
    return '';
  }
  const lines = contents
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const candidates = lines.filter((line) => !line.includes('='));
  return candidates.at(-1) || '';
}

const { arch, expectedMode, evidenceDir } = parseArgs(process.argv.slice(2));
const absoluteEvidenceDir = path.resolve(evidenceDir);
fs.mkdirSync(absoluteEvidenceDir, { recursive: true });

const preflightPath = path.join(absoluteEvidenceDir, 'signing-preflight.txt');
const bundlePath = path.join(absoluteEvidenceDir, 'bundle-check.txt');
const preflightContents = readOptionalFile(preflightPath);
const bundleContents = readOptionalFile(bundlePath);

const preflight = {
  collected: Boolean(preflightContents),
  values: parseKeyValueOutput(preflightContents),
  stderrHint: collectLastNonKeyValueLine(preflightContents),
};
const bundle = {
  collected: Boolean(bundleContents),
  values: parseKeyValueOutput(bundleContents),
  stderrHint: collectLastNonKeyValueLine(bundleContents),
};

const verdict = buildVerdict(expectedMode, preflight, bundle);
const generatedAt = new Date().toISOString();

const summary = {
  generatedAt,
  arch,
  expectedMode,
  verdict,
  preflight,
  bundle,
};

const summaryJsonPath = path.join(absoluteEvidenceDir, 'summary.json');
const summaryMarkdownPath = path.join(absoluteEvidenceDir, 'summary.md');
fs.writeFileSync(summaryJsonPath, `${JSON.stringify(summary, null, 2)}\n`);
fs.writeFileSync(
  summaryMarkdownPath,
  renderMarkdown({ arch, expectedMode, preflight, bundle, verdict, generatedAt })
);

console.log(`evidence_dir=${absoluteEvidenceDir}`);
console.log(`summary_json=${summaryJsonPath}`);
console.log(`summary_markdown=${summaryMarkdownPath}`);
console.log(`verdict=${verdict.status}`);
