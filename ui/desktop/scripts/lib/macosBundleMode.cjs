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

function resolveMacosBundleMode(env = process.env) {
  const signingOverride = readBoolOverride(env, 'GOOSE_DESKTOP_SIGN');
  const signingEnabled =
    signingOverride !== undefined ? signingOverride : Boolean(readTrimmed(env, 'APPLE_TEAM_ID'));
  const cookieEncryptionOverride = readBoolOverride(env, 'GOOSE_ENABLE_COOKIE_ENCRYPTION');
  const enableCookieEncryption =
    cookieEncryptionOverride !== undefined ? cookieEncryptionOverride : signingEnabled;

  return {
    signingEnabled,
    signingMode: signingEnabled ? 'signed' : 'local-preview',
    enableCookieEncryption,
    disableKeyringByDefault: !signingEnabled,
    shouldAdhocResign: !signingEnabled,
  };
}

module.exports = {
  resolveMacosBundleMode,
};
