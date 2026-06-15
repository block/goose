const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const repoRoot = path.resolve(__dirname, '..', '..', '..');
const desktopRoot = path.resolve(__dirname, '..');
const desktopBinDir = path.join(desktopRoot, 'src', 'bin');
const binaryName = process.platform === 'win32' ? 'goosed.exe' : 'goosed';
const defaultBuildProfile = process.platform === 'darwin' ? 'release' : 'debug';

function isFile(filePath) {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function getMtimeMs(filePath) {
  try {
    return fs.statSync(filePath).mtimeMs;
  } catch {
    return 0;
  }
}

function getSizeBytes(filePath) {
  try {
    return fs.statSync(filePath).size;
  } catch {
    return 0;
  }
}

function ensureBinDir() {
  fs.mkdirSync(desktopBinDir, { recursive: true });
}

function copyBinary(sourcePath) {
  const destinationPath = path.join(desktopBinDir, binaryName);
  fs.copyFileSync(sourcePath, destinationPath);

  if (process.platform !== 'win32') {
    fs.chmodSync(destinationPath, 0o755);
  }

  return destinationPath;
}

function getProfileCandidates() {
  const requestedProfile = process.env.GOOSED_BUILD_PROFILE?.trim();
  if (requestedProfile) {
    if (!['debug', 'release'].includes(requestedProfile)) {
      console.error(
        `[ensure-goosed] Unsupported GOOSED_BUILD_PROFILE value: ${requestedProfile}`
      );
      process.exit(1);
    }

    return requestedProfile === 'release' ? ['release', 'debug'] : ['debug', 'release'];
  }

  return defaultBuildProfile === 'release' ? ['release', 'debug'] : ['debug', 'release'];
}

function isRepoOwnedBinary(candidatePath) {
  const resolvedPath = path.resolve(candidatePath);
  const allowedPaths = [
    path.join(desktopBinDir, binaryName),
    path.join(repoRoot, 'target', 'debug', binaryName),
    path.join(repoRoot, 'target', 'release', binaryName),
  ].map((candidate) => path.resolve(candidate));

  return allowedPaths.includes(resolvedPath);
}

function stageExistingBinary() {
  const stagedPath = path.join(desktopBinDir, binaryName);
  const targetCandidates = getProfileCandidates()
    .map((profile) => path.join(repoRoot, 'target', profile, binaryName))
    .filter((candidate) => isFile(candidate));

  const preferredTarget = targetCandidates[0];

  if (preferredTarget) {
    if (
      !isFile(stagedPath) ||
      getMtimeMs(preferredTarget) > getMtimeMs(stagedPath) ||
      getSizeBytes(preferredTarget) !== getSizeBytes(stagedPath)
    ) {
      const destinationPath = copyBinary(preferredTarget);
      console.log(`[ensure-goosed] Staged ${preferredTarget} -> ${destinationPath}`);
      return destinationPath;
    }

    console.log(`[ensure-goosed] Using staged binary at ${stagedPath}`);
    return stagedPath;
  }

  if (isFile(stagedPath)) {
    console.log(`[ensure-goosed] Using staged binary at ${stagedPath}`);
    return stagedPath;
  }

  return null;
}

function buildGoosed() {
  const buildProfile = getProfileCandidates()[0];
  const cargoArgs = ['build', '-p', 'goose-server', '--bin', 'goosed'];
  if (buildProfile === 'release') {
    cargoArgs.splice(1, 0, '--release');
  }
  const registryMirror = process.env.GOOSE_CARGO_REGISTRY_MIRROR?.trim();

  if (registryMirror) {
    if (registryMirror !== 'rsproxy-cn') {
      console.error(
        `[ensure-goosed] Unsupported GOOSE_CARGO_REGISTRY_MIRROR value: ${registryMirror}`
      );
      process.exit(1);
    }

    cargoArgs.unshift(
      '--config',
      'source.rsproxy.registry="sparse+https://rsproxy.cn/index/"',
      '--config',
      'source.crates-io.replace-with="rsproxy"'
    );
    console.log('[ensure-goosed] Using rsproxy-cn mirror for cargo crate downloads');
  }

  console.log(
    `[ensure-goosed] Building ${buildProfile} goosed via cargo ${cargoArgs.join(' ')}`
  );
  const result = spawnSync('cargo', cargoArgs, {
    cwd: repoRoot,
    stdio: 'inherit',
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }

  const builtPath = path.join(repoRoot, 'target', buildProfile, binaryName);
  if (!isFile(builtPath)) {
    console.error(`[ensure-goosed] Expected built binary at ${builtPath}`);
    process.exit(1);
  }

  const destinationPath = copyBinary(builtPath);
  console.log(`[ensure-goosed] Staged ${builtPath} -> ${destinationPath}`);
  return destinationPath;
}

function main() {
  const overridePath = process.env.GOOSED_BINARY;
  if (overridePath) {
    if (!isFile(overridePath)) {
      console.error(`[ensure-goosed] Invalid GOOSED_BINARY path: ${overridePath}`);
      process.exit(1);
    }

    if (!isRepoOwnedBinary(overridePath)) {
      console.error(
        `[ensure-goosed] Refusing external GOOSED_BINARY outside repo-owned development paths: ${path.resolve(overridePath)}`
      );
      process.exit(1);
    }

    console.log(`[ensure-goosed] Using GOOSED_BINARY override at ${path.resolve(overridePath)}`);
    return;
  }

  ensureBinDir();

  if (stageExistingBinary()) {
    return;
  }

  buildGoosed();
}

main();
