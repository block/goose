---
title: Building goose in Docker
sidebar_label: goose in Docker
---

:::info Tell Us What You Need
There are various scenarios where you might want to build goose in Docker. If the instructions below do not meet your needs, please contact us by replying to our [discussion topic](https://github.com/block/goose/discussions/1496).
:::


You can build goose from the source file within a Docker container. This approach not only provides security benefits by creating an isolated environment but also enhances consistency and portability. For example, if you need to troubleshoot an error on a platform you don't usually work with (such as Ubuntu), you can easily debug it using Docker.

To begin, you will need to modify the `Dockerfile` and `docker-compose.yml` files to suit your requirements. Some changes you might consider include:

- **Required:** Setting your API key, provider, and model in the `docker-compose.yml` file as environment variables because the keyring settings do not work on Ubuntu in Docker. This example uses the Google API key and its corresponding settings, but you can [find your own list of supported providers and their API keys](https://github.com/block/goose/blob/main/ui/desktop/src/components/settings/providers/ProviderRegistry.tsx) in the provider registry.

- **Optional:** Changing the base image to a different Linux distribution in the `Dockerfile`. This example uses Ubuntu, but you can switch to another distribution such as CentOS, Fedora, or Alpine.

- **Optional:** Mounting your personal goose settings and hints files in the `docker-compose.yml` file. This allows you to use your personal settings and hints files within the Docker container.

 

After setting the credentials, you can build the Docker image using the following command:

```bash
docker-compose -f documentation/docs/docker/docker-compose.yml build
```

Next, run the container and connect to it using the following command:

```bash
docker-compose -f documentation/docs/docker/docker-compose.yml run --rm goose-cli
```

Inside the container, run the following command to configure goose:

```bash
goose configure
```

When prompted to save the API key to the keyring, select `No`, as you are already passing the API key as an environment variable.

Configure goose a second time, and this time, you can [add any extensions](/docs/getting-started/using-extensions) you need.

After that, you can start a session:

```bash
goose session
```

You should now be able to connect to goose with your configured extensions enabled.