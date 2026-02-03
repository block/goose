# Testing and Developing Locally

This guide covers the steps needed to set up the development environment for OpenLIT Stack using Docker Compose. Our setup includes two main components: `client`, both of which are orchestrated by a Docker Compose file named `dev-docker-compose.yml`.

## Running the Development Environment

To run the development environment, follow these steps:

1. Open a terminal and set context as this directory ()`/src`).

3. Use the following Docker Compose command to build and start the containers as specified in the `dev-docker-compose.yml` file:

    ```
    docker compose -f dev-docker-compose.yml up --build -d
    ```

    The `--build` flag ensures that Docker builds images for our services (if needed) before starting the containers.

4. Once the command completes, the `client` services should be running in containers, and you can access OpenLIT Client at 127.0.0.1:3000.

5. To stop the services and remove the containers, you can use the following command:

    ```
    docker compose -f dev-docker-compose.yml down
    ```

This command stops all running containers and removes them along with their networks. It's a clean way to shut down your development environment.
