<div align="center">
<img src="https://github.com/openlit/.github/blob/main/profile/assets/wide-logo-no-bg.png?raw=true" alt="OpenLIT Logo" width="30%">

#### Observability, Exceptions, Prompts, Vault, Playground

# Open Source Platform for AI Engineering

**[Documentation](https://docs.openlit.io/) | [Quickstart](-getting-started-with-llm-observability) | [Python SDK](https://github.com/openlit/openlit/tree/main/sdk/python) | [Typescript SDK](https://github.com/openlit/openlit/tree/main/sdk/typescript) |** 

**[Roadmap](#️-roadmap) | [Feature Request](https://github.com/openlit/openlit/issues/new?assignees=&labels=%3Araised_hand%3A+Up+for+Grabs%2C+%3Arocket%3A+Feature&projects=&template=feature-request.md&title=%5BFeat%5D%3A) | [Report a Bug](https://github.com/openlit/openlit/issues/new?assignees=&labels=%3Abug%3A+Bug%2C+%3Araised_hand%3A+Up+for+Grabs&projects=&template=bug.md&title=%5BBug%5D%3A)** 

[![OpenLIT](https://img.shields.io/badge/OpenLIT-orange)](https://openlit.io/)
[![License](https://img.shields.io/github/license/openlit/openlit?label=License&logo=github&color=f80&logoColor=white)](https://github.com/openlit/openlit/blob/main/LICENSE)
[![Downloads](https://static.pepy.tech/badge/openlit/month)](https://pepy.tech/project/openlit)
[![GitHub Last Commit](https://img.shields.io/github/last-commit/openlit/openlit)](https://github.com/openlit/openlit/pulse)
[![GitHub Contributors](https://img.shields.io/github/contributors/openlit/openlit)](https://github.com/openlit/openlit/graphs/contributors)

[![Slack](https://img.shields.io/badge/Slack-4A154B?logo=slack&logoColor=white)](https://join.slack.com/t/openlit/shared_invite/zt-2etnfttwg-TjP_7BZXfYg84oAukY8QRQ)
[![X](https://img.shields.io/badge/follow-%40openlit__io-1DA1F2?logo=x&style=social)](https://twitter.com/openlit_io)
</div>

---

![OpenLIT Banner](https://github.com/openlit/.github/blob/main/profile/assets/openlit-feature-banner.png?raw=true)

**OpenLIT** allows you to simplify your AI development workflow, especially for Generative AI and LLMs. It streamlines essential tasks like experimenting with LLMs, organizing and versioning prompts, and securely handling API keys. With just one line of code, you can enable **OpenTelemetry-native** observability, offering full-stack monitoring that includes LLMs, vector databases, and GPUs. This enables developers to confidently build AI features and applications, transitioning smoothly from testing to production.

This project proudly follows and maintains the [Semantic Conventions](https://github.com/open-telemetry/semantic-conventions/tree/main/docs/gen-ai) with the OpenTelemetry community, consistently updating to align with the latest standards in Observability.

## ⚡ Features

- 📈 **Analytics Dashboard**: Monitor your AI application's health and performance with detailed dashboards that track metrics, costs, and user interactions, providing a clear view of overall efficiency.

- 🔌 **OpenTelemetry-native Observability SDKs**: Vendor-neutral SDKs to send traces and metrics to your existing observability tools. 

- 💲 **Cost Tracking for Custom and Fine-Tuned Models**: Tailor cost estimations for specific models using custom pricing files for precise budgeting.

- 🔔 **Exceptions Monitoring Dashboard**: Quickly spot and resolve issues by tracking common exceptions and errors with a dedicated monitoring dashboard.

- 💭 **Prompt Management**: Manage and version prompts using Prompt Hub for consistent and easy access across applications.

- 🔑 **API Keys and Secrets Management**: Securely handle your API keys and secrets centrally, avoiding insecure practices.

- 🎮 **Experiemnt with different LLMs**: Use OpenGround to explore, test and compare various LLMs side by side.

## 🚀 Quick Start

Follow the steps below to get OpenLIT running in your environment. Both Docker and manual installation options are provided.

### Docker

1. **Pull the Docker image**
    ```bash
    docker pull ghcr.io/openlit/openlit-client:latest
    ```

2. **Run the container with Environment Variables**
    Here, replace `<YourValues>` with actual values for the environment variables.
    ```bash
    docker run -d -p 3000:3000 \
    -e INIT_DB_HOST="<ClickHouse-URL>" \
    -e INIT_DB_PORT="<ClickHouse-Port>" \
    -e INIT_DB_DATABASE="<ClickHouse-Database-name>" \
    -e INIT_DB_USERNAME="<ClickHouse-username>" \
    -e INIT_DB_PASSWORD="<ClickHouse-password>" \
    -e SQLITE_DATABASE_URL=file:/app/client/data/data.db \
    --name openlit-client ghcr.io/openlit/openlit-client:latest
    ```

3. Login to OpenLIT at `127.0.0.1:3000` using the default credentials and start monitoring and evaluating your LLM Applications
    - Email as `user@openlit.io`
    - Password as `openlituser`

You can also use the [OpenLIT Helm Chart](https://github.com/openlit/helm/tree/main/charts/openlit) to deploy OpenLIT Client in Kubernetes

### Manual Setup (Development)

1. **Clone the openlit repository**
    ```sh
    git clone https://github.com/openlit/openlit.git
    ```
2. **Navigate to the client folder**
    ```sh
    cd openlit/src/client
    ```
3. **Install the dependencies**
    ```sh
    npm install
    ```
4. **Configure Environment Variables**

    Below are the commands to set environment variables if needed during manual setup. Replace `<value>` with your actual configuration:
    ```sh
    export INIT_DB_HOST=<value>
    export INIT_DB_PORT=<value>
    export INIT_DB_DATABASE=<value>
    export INIT_DB_USERNAME=<value>
    export INIT_DB_PASSWORD=<value>
    export SQLITE_DATABASE_URL=<value>
    ```
    
5. **Apply the migrations**
    
    This creates your SQLite database schema.
    ```sh
    npx prisma migrate deploy
    npx prisma generate
    ```
6. **(Optional) Seed the database**
    
    If desired, seed the database to create a default user (`user@openlit.io` with the password `openlituser`) and database configuration.
    ```sh
    npx prisma db seed
    ```
    Ensure the database is empty before running this command.

7. **Start the development server**
    ```sh
    npm run dev
    ```
8. Login to OpenLIT at `127.0.0.1:3000` using the default credentials and start monitoring and evaluating your LLM Applications
    - Email as `user@openlit.io`
    - Password as `openlituser`

## Configuration

To configure OpenLIT Client, you can pass the following environment values, each tailored to suit your infrastructure and operational preferences. This customization allows OpenLIT Client to seamlessly integrate with your existing setup and respond to its demands effectively.


| Variable             | Description                                                                 | Required | Example                               |
|----------------------|-----------------------------------------------------------------------------|:--------:|---------------------------------------|
| `INIT_DB_HOST`       | Host address of the ClickHouse server to connect to.                        |    ✓     | `127.0.0.1`                           |
| `INIT_DB_PORT`       | Port on which ClickHouse listens.                                           |    ✓     | `8123`                                |
| `INIT_DB_DATABASE`   | Database name in ClickHouse for OpenLIT Client.                                |          | `default`                             |
| `INIT_DB_USERNAME`   | Username for authenticating with ClickHouse.                                |          | `default`                             |
| `INIT_DB_PASSWORD`   | Password for authenticating with ClickHouse.                                |          | `default`                             |
| `SQLITE_DATABASE_URL`| Location of the SQLITE database for OpenLIT Client data storage.               |    ✓     | `file:/app/client/data/data.db`       |

For more detailed information on configuration options and additional settings, please visit the OpenLIT documentation page: [OpenLIT Configuration Details](https://docs.openlit.io/latest/configuration).

## 🛣️ Roadmap

We are dedicated to continuously improving OpenLIT. Here's a look at what's been accomplished and what's on the horizon:

| Feature                                                                                      | Status        |
|----------------------------------------------------------------------------------------------|---------------|
| [OpenTelemetry-native Observability SDK for Tracing and Metrics](https://github.com/openlit/openlit/tree/text-upgrade/sdk/python) | ✅ Completed  |
| [OpenTelemetry-native GPU Monitoring](https://docs.openlit.io/latest/features/gpu)           | ✅ Completed  |
| [Exceptions and Error Monitoring]()                                                          | ✅ Completed  |
| [Prompt Hub for Managing and Versioning Prompts](https://docs.openlit.io/latest/features/prompt-hub) | ✅ Completed  |
| [OpenGround for Testing and Comparing LLMs]()                                                | ✅ Completed  |
| [Vault for Central Management of LLM API Keys and Secrets](https://docs.openlit.io/latest/features/vault) | ✅ Completed  |
| [Cost Tracking for Custom Models](https://docs.openlit.io/latest/features/pricing)           | ✅ Completed  |
| [Auto-Evaluation Metrics Based on Usage](https://github.com/openlit/openlit/issues/470)                                                   | 🔜 Coming Soon |
| [Human Feedback for LLM Events](https://github.com/openlit/openlit/issues/471)                                                            | 🔜 Coming Soon |
| [Dataset Generation Based on LLM Events](https://github.com/openlit/openlit/issues/472)                                                   | 🔜 Coming Soon |
| [Real-Time Guardrails Implementation]()                                                      | 📝 Planned    |

## 🌱 Contributing

Whether it's big or small, we love contributions 💚. Check out our [Contribution guide](./CONTRIBUTING.md) to get started

Unsure where to start? Here are a few ways to get involved:

- Join our [Slack](https://join.slack.com/t/openlit/shared_invite/zt-2etnfttwg-TjP_7BZXfYg84oAukY8QRQ) or [Discord](https://discord.gg/rjvTm6zd) community to discuss ideas, share feedback, and connect with both our team and the wider OpenLIT community.

Your input helps us grow and improve, and we're here to support you every step of the way.

[![OpenLIT - One click observability, evals for LLMs & GPUs | Product Hunt](https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=460690&theme=light)](https://www.producthunt.com/posts/openlit?embed=true&utm_source=badge-featured&utm_medium=badge&utm_souce=badge-openlit)
<a href="https://fazier.com/launches/openlit-2" target="_blank" rel="noopener noreferrer"><img src="https://fazier.com/api/v1/public/badges/embed_image.svg?launch_id=779&badge_type=daily" width="270" alt="Example Image" class="d-inline-block mt-3 p-3 rounded img-fluid" /></a>

## 💚 Community & Support

Connect with OpenLIT community and maintainers for support, discussions, and updates:

- 🌟 If you like it, Leave a star on our [GitHub](https://github.com/openlit/openlit/)
- 🌍 Join our [Slack](https://join.slack.com/t/openlit/shared_invite/zt-2etnfttwg-TjP_7BZXfYg84oAukY8QRQ) or [Discord](https://discord.gg/CQnXwNT3) community for live interactions and questions.
- 🐞 Report bugs on our [GitHub Issues](https://github.com/openlit/openlit/issues) to help us improve OpenLIT.
- 𝕏 Follow us on [X](https://twitter.com/openlit_io) for the latest updates and news.

## License

OpenLIT is available under the [Apache-2.0 license](LICENSE).
