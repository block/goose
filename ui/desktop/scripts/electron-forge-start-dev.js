const path = require('node:path');
const { spawn } = require('node:child_process');

function getDesktopRoot(scriptDir = __dirname) {
  return path.resolve(scriptDir, '..');
}

function getElectronForgeStartArgs(forwardedArgs = []) {
  return forwardedArgs.length > 0 ? ['start', '--', ...forwardedArgs] : ['start'];
}

function getElectronForgeCommand(platform = process.platform) {
  return platform === 'win32' ? 'electron-forge.cmd' : 'electron-forge';
}

function main(forwardedArgs = process.argv.slice(2)) {
  const desktopRoot = getDesktopRoot();
  const child = spawn(
    getElectronForgeCommand(),
    getElectronForgeStartArgs(forwardedArgs),
    {
      cwd: desktopRoot,
      env: process.env,
      shell: process.platform === 'win32',
      stdio: 'inherit',
    }
  );

  child.on('error', (error) => {
    console.error('[start-gui]', error.message);
    process.exit(1);
  });

  child.on('exit', (code) => {
    process.exit(code ?? 0);
  });
}

if (require.main === module) {
  main();
}

module.exports = {
  getDesktopRoot,
  getElectronForgeCommand,
  getElectronForgeStartArgs,
  main,
};
