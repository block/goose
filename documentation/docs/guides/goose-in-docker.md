---
title: Goose in Docker
sidebar_position: 3
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

:::info Tell Us What You Need
There are various scenarios where you might want to build Goose in Docker. If the instructions below do not meet your needs, please contact us by replying to our [discussion topic](https://github.com/block/goose/discussions/1496).
:::
# Use Case 1: Building Goose from the source in Docker

As a Goose user and developer, you can build Goose from the source file within a Docker container. This approach not only provides security benefits by creating an isolated environment but also enhances consistency and portability. For example, if you need to troubleshoot an error on a platform you don't usually work with (such as Ubuntu), you can easily debug it using Docker.

To begin, you will need to modify the `Dockerfile` and `docker-compose.yml` files to suit your requirements. Some changes you might consider include:
- Setting your API key, provider, and model in the `docker-compose.yml` file. Our example uses the Google API key and its corresponding settings, but you can find your own list of API keys [here](https://github.com/block/goose/blob/main/ui/desktop/src/components/settings/models/hardcoded_stuff.tsx#L86-L94) and the corresponding settings [here](https://github.com/block/goose/blob/main/ui/desktop/src/components/settings/models/hardcoded_stuff.tsx#L67-L77).
- Changing the base image to a different Linux distribution in the `Dockerfile`. Our example uses Ubuntu, but you can switch to another distribution such as CentOS, Fedora, or Alpine.
- Mounting your personal Goose settings and hints files in the `docker-compose.yml` file. This allows you to use your personal settings and hints files within the Docker container.

Among these, only the first change is mandatory. We need to set the API key, provider, and model as environment variables because the keyring settings do not work on Ubuntu in Docker.

After setting the credentials, you can build the Docker image using the following command:

```bash
docker-compose -f documentation/docs/docker/docker-compose.yml build
```

Next, run the container and connect to it using the following command:

```bash
docker-compose -f documentation/docs/docker/docker-compose.yml run --rm goose-cli
```
Inside the container, first try to run the following command to configure Goose:

```bash
goose configure
```
When prompted to save the API key to the keyring, select No, as we are already passing the API key as an environment variable.

Then, you can configure Goose a second time, and this time, you can add extensions:
```bash
goose configure
```
For example, you can add the `Developer` extension. After that, you can start a session:
```bash
goose session
```
You should now be able to connect to Goose with the developer extension enabled. Follow the other tutorials if you want to enable more extensions.