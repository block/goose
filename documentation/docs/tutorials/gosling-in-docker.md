---
title: gosling in Docker
sidebar_label: gosling in Docker
description: Run gosling inside Docker containers, or run extensions in existing containers for devcontainer workflows
---

This guide covers two Docker-related scenarios:
1. **Running gosling inside Docker** - Build and run the gosling process itself in a container
2. **Running extensions in Docker** - Run gosling on your host but execute extensions inside a container

## Running gosling Inside Docker

You can build gosling from the source file within a Docker container. This approach not only provides security benefits by creating an isolated environment but also enhances consistency and portability. For example, if you need to troubleshoot an error on a platform you don't usually work with (such as Ubuntu), you can easily debug it using Docker.

To begin, you will need to modify the [`Dockerfile` and `docker-compose.yml` files](https://github.com/repo-makeover/gosling/tree/main/documentation/docs/docker) to suit your requirements. Some changes you might consider include:

- **Required:** Setting your API key, provider, and model in the `docker-compose.yml` file as environment variables because the keyring settings do not work on Ubuntu in Docker. This example uses Google Gemini.

- **Optional:** Changing the base image to a different Linux distribution in the `Dockerfile`. This example uses Ubuntu, but you can switch to another distribution such as CentOS, Fedora, or Alpine.

- **Optional:** Mounting your personal gosling settings and hints files in the `docker-compose.yml` file. This allows you to use your personal settings and hints files within the Docker container.

:::tip Automated Alternative
For an automated approach to running gosling in containers, see the [Container-Use MCP extension](/docs/mcp/container-use-mcp), which creates and manages containers for you through conversation.
:::

After setting the credentials, you can build the Docker image using the following command:

```bash
docker-compose -f documentation/docs/docker/docker-compose.yml build
```

Next, run the container and connect to it using the following command:

```bash
docker-compose -f documentation/docs/docker/docker-compose.yml run --rm gosling-cli
```

Inside the container, run the following command to configure gosling:

```bash
gosling configure
```

When prompted to save the API key to the keyring, select `No`, as you are already passing the API key as an environment variable.

Configure gosling a second time, and this time, you can [add any extensions](/docs/getting-started/using-extensions) you need.

After that, you can start a session:

```bash
gosling session
```

You should now be able to connect to gosling with your configured extensions enabled.

## Running Extensions in Docker Containers

The `--container` flag allows you to run gosling extensions inside your Docker containers.

### Usage

```bash
gosling session --container <container-id-or-name>
```

Extensions configured in your `config.yaml` will automatically run inside the specified container. Find your container ID or name with `docker ps`.

### Requirements

- Extensions must exist in the container and be accessible via the same paths used in your extension config
- To run built-in extensions, the gosling CLI must be [installed](/docs/getting-started/installation) inside the container

### Examples

```bash
# Start an interactive session with extensions from config.yaml
gosling session --container my-dev-container

# Start a non-interactive session with instructions
gosling run --container my-dev-container --text "your instructions here"

# Specify an extension to run in the container
gosling session --container 4c76a1beed85 --with-extension "uvx mcp-server-fetch"

# Workaround: Use full path if container can't find the command
gosling session --container 4c76a1beed85 --with-extension "/root/.local/bin/uvx mcp-server-fetch"
```
