function readTrimmed(env, key) {
  const value = env[key];
  return typeof value === 'string' ? value.trim() : '';
}

function readBoolOverride(env, key) {
  const value = readTrimmed(env, key).toLowerCase();
  if (value === 'true') {
    return true;
  }
  if (value === 'false') {
    return false;
  }
  return undefined;
}

function inspectMacosSigningReadiness(env = process.env) {
  const explicitSigning = readBoolOverride(env, 'GOOSE_DESKTOP_SIGN');
  const signingRequested =
    explicitSigning !== undefined ? explicitSigning : Boolean(readTrimmed(env, 'APPLE_TEAM_ID'));
  const requestedMode = signingRequested ? 'signed' : 'local-preview';

  const requiredSecrets = [
    'APPLE_CERTIFICATE_BASE64',
    'APPLE_CERTIFICATE_PASSWORD',
    'APPLE_TEAM_ID',
    'APPLE_ID',
    'APPLE_ID_PASSWORD',
  ];

  const presentSecrets = Object.fromEntries(
    requiredSecrets.map((key) => [key, readTrimmed(env, key).length > 0])
  );
  const missingSecrets = requiredSecrets.filter((key) => !presentSecrets[key]);

  let certificateBase64Decodes = null;
  if (presentSecrets.APPLE_CERTIFICATE_BASE64) {
    try {
      certificateBase64Decodes = Buffer.from(env.APPLE_CERTIFICATE_BASE64, 'base64').length > 0;
    } catch {
      certificateBase64Decodes = false;
    }
  }

  const invalidSecrets =
    certificateBase64Decodes === false ? ['APPLE_CERTIFICATE_BASE64'] : [];
  const readyForSignedRelease =
    signingRequested && missingSecrets.length === 0 && invalidSecrets.length === 0;

  const blockers = [];
  if (signingRequested && missingSecrets.length > 0) {
    blockers.push(`missing secrets: ${missingSecrets.join(', ')}`);
  }
  if (signingRequested && invalidSecrets.length > 0) {
    blockers.push(`invalid secrets: ${invalidSecrets.join(', ')}`);
  }

  const fallbackMessage = signingRequested
    ? 'Fallback to local-preview by rerunning with GOOSE_DESKTOP_SIGN=false and using bundle:default or bundle:intel.'
    : 'Local preview remains available through bundle:default or bundle:intel.';

  return {
    signingRequested,
    requestedMode,
    requiredSecrets,
    presentSecrets,
    missingSecrets,
    invalidSecrets,
    readyForSignedRelease,
    fallbackMessage,
    externalConditions: [
      'Valid Developer ID Application certificate',
      'Apple Developer account with notarization access',
      'Runner network access to Apple notarization services',
    ],
    blockerSummary: blockers.join('; '),
  };
}

module.exports = {
  inspectMacosSigningReadiness,
};
