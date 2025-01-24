const { FusesPlugin } = require('@electron-forge/plugin-fuses');
const { FuseV1Options, FuseVersion } = require('@electron/fuses');

let cfg = {
  asar: true,
  extraResource: ['src/bin', 'src/images'],
  icon: 'src/images/icon',
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
  protocols: [
    {
      name: "GooseProtocol",     // The macOS CFBundleURLName
      schemes: ["goose"]         // The macOS CFBundleURLSchemes array
    }
  ]
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
      name: '@electron-forge/maker-squirrel',
      config: {},
    },
    {
      name: '@electron-forge/maker-zip',
      platforms: ['darwin'],
    },
    {
      name: '@electron-forge/maker-deb',
      config: {},
    },
    {
      name: '@electron-forge/maker-rpm',
      config: {},
    },
  ],
  plugins: [
    {
      name: '@electron-forge/plugin-vite',
      config: {
        // `build` can specify multiple entry builds, which can be Main process, Preload scripts, Worker process, etc.
        // If you are familiar with Vite configuration, it will look really familiar.
        build: [
          {
            // `entry` is just an alias for `build.lib.entry` in the corresponding file of `config`.
            entry: 'src/main.ts',
            config: 'vite.main.config.ts',
            target: 'main',
          },
          {
            entry: 'src/preload.js',
            config: 'vite.preload.config.ts',
            target: 'preload',
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
