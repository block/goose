const { FusesPlugin } = require('@electron-forge/plugin-fuses');
const { FuseV1Options, FuseVersion } = require('@electron/fuses');
const { resolve } = require('path');

let cfg = {
  asar: true,
  extraResource: ['src/bin', 'src/images'],
  icon: 'src/images/icon',
  // Windows specific configuration
  win32: {
    icon: 'src/images/icon.ico',
    certificateFile: process.env.WINDOWS_CERTIFICATE_FILE,
    certificatePassword: process.env.WINDOWS_CERTIFICATE_PASSWORD,
    rfc3161TimeStampServer: 'http://timestamp.digicert.com',
    signWithParams: '/fd sha256 /tr http://timestamp.digicert.com /td sha256'
  },
  // Protocol registration
  protocols: [
    {
      name: "GooseProtocol",
      schemes: ["goose"]
    }
  ],
  // macOS specific configuration
  osxSign: {
    entitlements: 'entitlements.plist',
    'entitlements-inherit': 'entitlements.plist',
    'gatekeeper-assess': false,
    hardenedRuntime: true,
    identity: 'Developer ID Application: Michael Neale (W2L75AE9HQ)',
  },
  osxNotarize: {
    appleId: process.env['APPLE_ID'],
    appleIdPassword: process.env['APPLE_ID_PASSWORD'],
    teamId: process.env['APPLE_TEAM_ID']
  },
}

if (process.env['APPLE_ID'] === undefined) {
  delete cfg.osxNotarize;
  delete cfg.osxSign;
}

module.exports = {
  packagerConfig: cfg,
  rebuildConfig: {},
  makers: [
    {
      name: '@electron-forge/maker-zip',
      platforms: ['darwin', 'win32'],
      config: {
        arch: process.env.ELECTRON_ARCH === 'x64' ? ['x64'] : ['arm64'],
        options: {
          icon: 'src/images/icon.ico'
        }
      }
    },
    {
      name: '@electron-forge/maker-deb',
      config: {
        name: 'Goose',
        bin: 'Goose'
      },
    },
    {
      name: '@electron-forge/maker-rpm',
      config: {
        name: 'Goose',
        bin: 'Goose'
      },
    },
  ],
  plugins: [
    {
      name: '@electron-forge/plugin-vite',
      config: {
        build: [
          {
            entry: 'src/main.ts',
            config: 'vite.main.config.ts',
          },
          {
            entry: 'src/preload.ts',
            config: 'vite.preload.config.ts',
          },
        ],
        renderer: [
          {
            name: 'main_window',
            config: 'vite.renderer.config.ts',
          },
        ],
      },
    },
    // Fuses are used to enable/disable various Electron functionality
    // at package time, before code signing the application
    new FusesPlugin({
      version: FuseVersion.V1,
      [FuseV1Options.RunAsNode]: false,
      [FuseV1Options.EnableCookieEncryption]: true,
      [FuseV1Options.EnableNodeOptionsEnvironmentVariable]: false,
      [FuseV1Options.EnableNodeCliInspectArguments]: false,
      [FuseV1Options.EnableEmbeddedAsarIntegrityValidation]: true,
      [FuseV1Options.OnlyLoadAppFromAsar]: true,
    }),
  ],
};
