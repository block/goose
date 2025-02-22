# Contribution Guide

Goose is open source! 

We welcome pull requests for general contributions! If you have a larger new feature or any questions on how to develop a fix, we recommend you open an issue before starting.

>[!TIP] Beyond code, check out [other ways to contribute](#other-ways-to-contribute)

## Prerequisites

Goose includes rust binaries alongside an electron app for the GUI. To work
on the rust backend, you will need to [install rust and cargo][rustup]. To work
on the App, you will also need to [install node and npm][nvm] - we recommend through nvm.

We provide a shortcut to standard commands using [just][just] in our `justfile`.

## Getting Started

### Rust

First let's compile goose and try it out

```
cargo build
```

when that is done, you should now have debug builds of the binaries like the goose cli:

```
./target/debug/goose --help
```

If you haven't used the CLI before, you can use this compiled version to do first time configuration:

```
./target/debug/goose configure
```

And then once you have a connection to an LLM provider working, you can run a session!

```
./target/debug/goose session
```

These same commands can be recompiled and immediately run using `cargo run -p goose-cli` for iteration.
As you make changes to the rust code, you can try it out on the CLI, or also run checks and tests:

```
cargo check  # do your changes compile
cargo test  # do the tests pass with your changes.
```

### Node

Now let's make sure you can run the app.

```
just run-ui
```

The start gui will both build a release build of rust (as if you had done `cargo build -r`) and start the electron process.
You should see the app open a window, and drop you into first time setup. When you've gone through the setup,
you can talk to goose!

You can now make changes in the code in ui/desktop to iterate on the GUI half of goose.

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

If you encounter any issues during this process or have any questions, please reach out by opening an issue [here][issues], and we'll be happy to help.

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

## Enable traces in Goose with [locally hosted Langfuse](https://langfuse.com/docs/deployment/self-host)

- Run `just langfuse-server` to start your local Langfuse server. It requires Docker.
- Go to http://localhost:3000 and log in with the default email/password output by the shell script (values can also be found in the `.env.langfuse.local` file).
- Set the environment variables so that rust can connect to the langfuse server

```
export LANGFUSE_INIT_PROJECT_PUBLIC_KEY=publickey-local
export LANGFUSE_INIT_PROJECT_SECRET_KEY=secretkey-local
```

Then you can view your traces at http://localhost:3000

## Conventional Commits

This project follows the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification for PR titles. Conventional Commits make it easier to understand the history of a project and facilitate automation around versioning and changelog generation.

[issues]: https://github.com/block/goose/issues
[rustup]: https://doc.rust-lang.org/cargo/getting-started/installation.html
[nvm]: https://github.com/nvm-sh/nvm
[just]: https://github.com/casey/just?tab=readme-ov-file#installation


## Other Ways to Contribute

There are numerous ways to be an open source contributor and contribute to Goose. We're here to help you on your way! Here are some suggestions to get started. If you have any questions or need help, feel free to reach out to us on [Discord](https://discord.gg/block-opensource).

- **Stars on GitHub:** If you resonate with our project and find it valuable, consider starring our Goose on GitHub! 🌟
- **Ask Questions:** Your questions not only help us improve but also benefit the community. If you have a question, don't hesitate to ask it on [Discord](https://discord.gg/block-opensource).
- **Give Feedback:** Have a feature you want to see or encounter an issue with Goose, [click here to open an issue](https://github.com/block/goose/issues/new/choose), [start a discussion](https://github.com/block/goose/discussions) or tell us on Discord.
- **Participate in Community Events:** We host a variety of community events and livestreams on Discord every month, ranging from workshops to brainstorming sessions. You can subscribe to our [events calendar](https://calget.com/c/t7jszrie) or follow us on [social media](https://linktr.ee/blockopensource) to stay in touch.
- **Improve Documentation:** Good documentation is key to the success of any project. You can help improve the quality of our existing docs or add new pages.
- **Help Other Members** See another community member stuck? Or a contributor blocked by a question you know the answer to? Reply to community threads or do a code review for others to help.
- **Showcase Your Work:** Working on a project or written a blog post recently? Share it with the community in our [#share-your-work](https://discord.com/channels/1287729918100246654/1287729920797179958) channel.
- **Give Shoutouts:** Is there a project you love or a community/staff who's been especially helpful? Feel free to give them a shoutout in our [#general](https://discord.com/channels/1287729918100246654/1287729920797179957) channel.
- **Spread the Word:** Help us reach more people by sharing Goose's project, website, YouTube, and/or Twitter/X.