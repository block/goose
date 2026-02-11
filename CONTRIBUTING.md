# Contribution Guide

goose is open source!

We welcome pull requests for general contributions! If you have a larger new feature or any questions on how to develop a fix, we recommend you open an issue before starting.

> [!TIP]
> Beyond code, check out [other ways to contribute](#other-ways-to-contribute)

--- 

## 🤖 Quick Responsible AI Tips

If you use Goose, Copilot, Claude, or other AI tools to help with your PRs:  

**✅ Good Uses** 

- Boilerplate code and common patterns  
- Test generation  
- Docs and comments  
- Refactoring for clarity  
- Utility functions/helpers  

**❌ Avoid AI For** 

- Security-critical logic  
- Complex business rules you don’t understand  
- Large architectural or schema changes  

**Quality Checklist**  

- Understand every line of code you submit  
- All tests pass locally  
- Code follows Goose’s patterns  
- Document your changes  
- Ask for review if security or core code is involved  

👉 Full guide here: [Responsible AI-Assisted Coding Guide](./HOWTOAI.md)

---

## Prerequisites

goose includes Rust binaries alongside an electron app for the GUI.

We use [Hermit][hermit] to manage development dependencies (Rust, Node, npm, just, etc.).
Activate Hermit when entering the project:

```bash
source bin/activate-hermit
```

Or add [shell hook auto-activation](https://cashapp.github.io/hermit/usage/shell/#shell-hooks) so Hermit activates automatically when you `cd` into the project (recommended).

We provide a shortcut to standard commands using [just][just] in our `justfile`.

### Windows Subsystem for Linux

For WSL users, you might need to install `build-essential` and `libxcb` otherwise you might run into `cc` linking errors (cc stands for C Compiler).
Install them by running these commands:

```
sudo apt update                   # Refreshes package list (no installs yet)
sudo apt install build-essential  # build-essential is a package that installs all core tools
sudo apt install libxcb1-dev      # libxcb1-dev is the development package for the X C Binding (XCB) library on Linux
```

## Getting Started

### Rust

First let's compile goose and try it out
Since goose requires Hermit for managing dependencies, let's activate hermit.

```
cd goose
source ./bin/activate-hermit
cargo build
```

When that completes, debug builds of the binaries are available, including the goose CLI:

```
./target/debug/goose --help
```

For first-time setup, run the configure command:

```
./target/debug/goose configure
```

Once a connection to an LLM provider is working, start a session:

```
./target/debug/goose session
```

These same commands can be recompiled and immediately run using `cargo run -p goose-cli` for iteration.
When making changes to the Rust code, test them on the CLI or run checks, tests, and the linter:

```
cargo check  # verify changes compile
cargo test  # run tests with changes
cargo fmt   # format code
cargo clippy --all-targets -- -D warnings # run the linter
```

### Node

To run the app:

```
just run-ui
```

This command builds a release build of Rust (equivalent to `cargo build -r`) and starts the Electron process.
The app opens a window and displays first-time setup. After completing setup, goose is ready for use.

Make GUI changes in `ui/desktop`.

### Regenerating the OpenAPI schema

The file `ui/desktop/openapi.json` is automatically generated during the build.
It is written by the `generate_schema` binary in `crates/goose-server`.
To update the spec without starting the UI, run:

```
just generate-openapi
```

This command regenerates `ui/desktop/openapi.json` and then runs the UI's
`generate-api` script to rebuild the TypeScript client from that spec.

API changes should be made in the Rust source under `crates/goose-server/src/`.

### Debugging

To debug the Goose server, run it from an IDE. The configuration will depend on the IDE. The command to run is:

```
export GOOSE_SERVER__SECRET_KEY=test
cargo run --package goose-server --bin goosed -- agent   # or: `just run-server`
```

The server listens on port `3000` by default; this can be changed by setting the
`GOOSE_PORT` environment variable.

Once the server is running, start a UI and connect it to the server by running:

```
just debug-ui
```

The UI connects to the server started in the IDE, allowing breakpoints
and stepping through the server code while interacting with the UI.

## Creating a fork

To fork the repository:

1. Go to https://github.com/block/goose and click “Fork” (top-right corner).
2. This creates https://github.com/<your-username>/goose under your GitHub account.
3. Clone your fork (not the main repo):

```
git clone https://github.com/<your-username>/goose.git
cd goose
```

4. Add the main repository as upstream:

```
git remote add upstream https://github.com/block/goose.git
```

5. Create a branch in your fork for your changes:

```
git checkout -b my-feature-branch
```

6. Sync your fork with the main repo:

```
git fetch upstream

# Merge them into your local branch (e.g., 'main' or 'my-feature-branch')
git checkout main
git merge upstream/main
```

7. Push to your fork. Because you’re the owner of the fork, you have permission to push here.

```
git push origin my-feature-branch
```

8. Open a Pull Request from your branch on your fork to block/goose’s main branch.

## Keeping Your Fork Up-to-Date

To ensure a smooth integration of your contributions, it's important that your fork is kept up-to-date with the main repository. This helps avoid conflicts and allows us to merge your pull requests more quickly. Here’s how you can sync your fork:

### Syncing Your Fork with the Main Repository

1. **Add the Main Repository as a Remote** (Skip if you have already set this up):

   ```bash
   git remote add upstream https://github.com/block/goose.git
   ```

2. **Fetch the Latest Changes from the Main Repository**:

   ```bash
   git fetch upstream
   ```

3. **Checkout Your Development Branch**:

   ```bash
   git checkout your-branch-name
   ```

4. **Merge Changes from the Main Branch into Your Branch**:

   ```bash
   git merge upstream/main
   ```

   Resolve any conflicts that arise and commit the changes.

5. **Push the Merged Changes to Your Fork**:

   ```bash
   git push origin your-branch-name
   ```

This process will help you keep your branch aligned with the ongoing changes in the main repository, minimizing integration issues when it comes time to merge!

### Before Submitting a Pull Request

Before you submit a pull request, please ensure your fork is synchronized as described above. This check ensures your changes are compatible with the latest in the main repository and streamlines the review process.

If you encounter any issues during this process or have any questions, please reach out by [opening an issue][issues], and we'll be happy to help.

## Env Vars

You may want to make more frequent changes to your provider setup or similar to test things out
as a developer. You can use environment variables to change things on the fly without redoing
your configuration.

> [!TIP]
> At the moment, we are still updating some of the CLI configuration to make sure this is
> respected.

You can change the provider goose points to via the `GOOSE_PROVIDER` env var. If you already
have a credential for that provider in your keychain from previously setting up, it should
reuse it. For things like automations or to test without doing official setup, you can also
set the relevant env vars for that provider. For example `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`,
or `DATABRICKS_HOST`. Refer to the provider details for more info on required keys.

### Isolating Test Environments

When testing changes or running multiple goose configurations, use `GOOSE_PATH_ROOT` to isolate your data:

```bash
# Test with a clean environment
export GOOSE_PATH_ROOT="/tmp/goose-test"
./target/debug/goose session

# Or for a single command
GOOSE_PATH_ROOT="/tmp/goose-dev" cargo run -p goose-cli -- session
```

This creates isolated `config/`, `data/`, and `state/` directories under the specified path, preventing your test sessions from affecting your main goose installation. See the [environment variables guide](./documentation/docs/guides/environment-variables.md#development--testing) for more details.

## Enable traces in goose with [locally hosted Langfuse](https://langfuse.com/docs/deployment/self-host)

- [Start a local Langfuse using the docs](https://langfuse.com/self-hosting/docker-compose). Create an organization and project and create API credentials.
- Set the environment variables so that goose can connect to the langfuse server:

```
export LANGFUSE_INIT_PROJECT_PUBLIC_KEY=publickey-local
export LANGFUSE_INIT_PROJECT_SECRET_KEY=secretkey-local
```

Then you can view your traces at http://localhost:3000

## Conventional Commits

This project follows the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification for PR titles. Conventional Commits make it easier to understand the history of a project and facilitate automation around versioning and changelog generation.

[issues]: https://github.com/block/goose/issues
[hermit]: https://cashapp.github.io/hermit/
[just]: https://github.com/casey/just?tab=readme-ov-file#installation

## Developer Certificate of Origin

This project requires a [Developer Certificate of Origin](https://en.wikipedia.org/wiki/Developer_Certificate_of_Origin) sign-offs on all commits. This is a statement indicating that you are allowed to make the contribution and that the project has the right to distribute it under its license. When you are ready to commit, use the `--signoff` or `-s` flag to attach the sign-off to your commit.

```
git commit --signoff ...
# OR
git commit -s ...
```

## Other Ways to Contribute

There are numerous ways to be an open source contributor and contribute to goose. We're here to help you on your way! Here are some suggestions to get started. If you have any questions or need help, feel free to reach out to us on [Discord](https://discord.gg/goose-oss).

- **Stars on GitHub:** If you resonate with our project and find it valuable, consider starring our goose on GitHub! 🌟
- **Ask Questions:** Your questions not only help us improve but also benefit the community. If you have a question, don't hesitate to ask it on [Discord](https://discord.gg/goose-oss).
- **Give Feedback:** Have a feature you want to see or encounter an issue with goose, [click here to open an issue](https://github.com/block/goose/issues/new/choose), [start a discussion](https://github.com/block/goose/discussions) or tell us on Discord.
- **Participate in Community Events:** We host a variety of community events and livestreams on Discord every month, ranging from workshops to brainstorming sessions. You can subscribe to our [events calendar](https://calget.com/c/t7jszrie) or follow us on [social media](https://linktr.ee/goose_oss) to stay in touch.
- **Improve Documentation:** Good documentation is key to the success of any project. You can help improve the quality of our existing docs or add new pages.
- **Help Other Members:** See another community member stuck? Or a contributor blocked by a question you know the answer to? Reply to community threads or do a code review for others to help.
- **Showcase Your Work:** Working on a project or written a blog post recently? Share it with the community in our [#share-your-work](https://discord.com/channels/1287729918100246654/1287729920797179958) channel.
- **Give Shoutouts:** Is there a project you love or a community/staff who's been especially helpful? Feel free to give them a shoutout in our [#general](https://discord.com/channels/1287729918100246654/1287729920797179957) channel.
- **Spread the Word:** Help us reach more people by sharing goose's project, website, YouTube, and/or Twitter/X.
