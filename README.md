<h1 align="center">
Goose is your on-machine developer agent, working for you, on your terms
</h1>

<p align="center">
  <a href="https://opensource.org/licenses/Apache-2.0">
    <img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg">
  </a>
  <a href="https://discord.gg/7GaTvbDwga">
    <img src="https://img.shields.io/discord/1287729918100246654?logo=discord&logoColor=white&label=Join+Us&color=blueviolet" alt="Discord">
  </a>
</p>

## GOOSE 1.0!!

This is the branch for goose 1.0 WIP: which is a port over from python to rust + typescript/electron for optional desktop environment. WATCH THIS SPACE

## Building

```sh
cargo build
```

## Running the CLI

### Configure

```
goose configure
```

### Add/Remove system

```
goose system add <system_url>
goose system remove <system_url>
```

#### OpenAI provider (default):

```
export OPENAI_API_KEY=...

cargo run --bin goose -- session
```

#### Databricks provider (TODO):

```
export DATABRICKS_HOST=...
export DATABRICKS_TOKEN=...

cargo run --bin goose -- session
```

### Headless mode

Run goose once-off with instructions from a file

```
Usage: cargo run --bin goose -- run -i instructions.md
```

## GUI

Goose has an electron based GUI which you can see in `ui/desktop`:

<img width="732" alt="image" src="https://github.com/user-attachments/assets/17499ae5-7812-46f0-8aae-e4d3d9583c34">
<img width="739" alt="image" src="https://github.com/user-attachments/assets/13ff2304-8468-47e0-9de8-89d23a62ec26">
<img width="744" alt="image" src="https://github.com/user-attachments/assets/3a825455-6cd1-406b-a459-e2c73dba024b">

## Start sub system server

```sh
cd crates/stub-system
cargo run
```

## Troubleshooting

#### Compiling `tokenizers` library

`tokenizers` depends on `esaxx-rs` which failed to compile because 'cstdint' file
was not found. The following fixed it:

```

export CXXFLAGS="-isystem $(xcrun --show-sdk-path)/usr/include/c++/v1"
cargo check
```
