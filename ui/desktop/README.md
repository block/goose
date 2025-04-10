# Goose Desktop App

Native desktop app for Goose built with [Electron](https://www.electronjs.org/) and [ReactJS](https://react.dev/). 

# Prerequisites
- [nvm](https://github.com/nvm-sh/nvm) (recommended for managing node versions easier but not required)
- node major version equal to or greater than specified in .nvmrc and package.json
- [rust](https://www.rust-lang.org/tools/install) (for building the server)

```
git clone git@github.com:block/goose.git
cd goose/ui/desktop
nvm use
npm install
npm run start
```

# Building notes

This is an electron forge app, using vite and react.js. `gooosed` runs as multi process binaries on each window/tab similar to chrome.

see `package.json`: 

`npm run bundle:default` will give you a Goose.app/zip which is signed/notarized but only if you setup the env vars as per `forge.config.ts` (you can empty out the section on osxSign if you don't want to sign it) - this will have all defaults.

`npm run bundle:preconfigured` will make a Goose.app/zip signed and notarized, but use the following:

```python
            f"        process.env.GOOSE_PROVIDER__TYPE = '{os.getenv("GOOSE_BUNDLE_TYPE")}';",
            f"        process.env.GOOSE_PROVIDER__HOST = '{os.getenv("GOOSE_BUNDLE_HOST")}';",
            f"        process.env.GOOSE_PROVIDER__MODEL = '{os.getenv("GOOSE_BUNDLE_MODEL")}';"
```

This allows you to set for example GOOSE_PROVIDER__TYPE to be "databricks" by default if you want (so when people start Goose.app - they will get that out of the box). There is no way to set an api key in that bundling as that would be a terrible idea, so only use providers that can do oauth (like databricks can), otherwise stick to default goose.


# Running with goosed server from source

Set `VITE_START_EMBEDDED_SERVER=yes` to no in `.env`.
Run `cargo run -p goose-server` from parent dir.
`npm run start` will then run against this.
You can try server directly with `./test.sh`
