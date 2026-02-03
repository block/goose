# Contributing to OpenLIT

We welcome contributions to the OpenLIT project and are grateful for every contribution from bug reports to new features. If you are looking to contribute to the codebase, improve documentation, report issues, or suggest new features, this document is a set of guidelines to help you get started.

## Getting Started

Before you begin:
- Ensure you have a [GitHub account](https://github.com/join).
- Familiarize yourself with git commands in the [Git Documentation](https://git-scm.com/documentation).
- Read the README for project setup instructions.
- For Fleet Hub and OpAMP development, review the [OpAMP Deployment Guide](OPAMP_DEPLOYMENT.md) and [Certificate Management Guide](src/opamp-server/CERTIFICATES.md).

## Contributing Workflow

Here's how you can contribute to OpenLIT:

1. **Fork the Repository**
   - Click the "Fork" button at the top right corner of the [OpenLIT repository](https://github.com/openlit/openlit).

2. **Clone the Forked Repository**
   - Clone your fork to your local machine:

     ```
     git clone https://github.com/YOUR_USERNAME/openlit.git
     ```

3. **Create Your Feature Branch**
   - Create a branch for your contribution:

     ```
     git checkout -b feature/my-new-feature
     ```

4. **Make Your Changes**
   - Make and test your changes locally. Make sure you adhere to the code style and guidelines of the project.
   - For OpAMP-related development, refer to the [OpAMP Deployment Guide](OPAMP_DEPLOYMENT.md) for setup and configuration.
   - For TLS certificate management in OpAMP, see the [Certificate Management Guide](src/opamp-server/CERTIFICATES.md).

5. **Commit Your Changes**
   - Commit your changes with a descriptive message:

     ```
     git commit -am 'Add some feature'
     ```

6. **Push to GitHub**
   - Push your changes to your fork:

     ```
     git push origin feature/my-new-feature
     ```

7. **Submit a Pull Request**
   - Go to your fork on GitHub, select your feature branch, and click on "Pull request" to send a pull request to the original repository.
   - Ensure the description clearly describes the problem and solution. Include any relevant issue numbers in your pull request description.

## Reporting Issues

If you find a bug or have a suggestion for improvement:
- Check the GitHub Issue Tracker to see if the issue has already been reported.
- If the issue is new, click the "Issues" tab, and then click "New Issue" to submit your issue.
- Provide as much information as possible to help us resolve the issue. This can include error messages, screenshots, and the steps to reproduce the issue.

## Pull Request Guidelines

When submitting a pull request, please follow these guidelines for a smooth collaboration process:
- Keep changes compact and well-documented to make the review process easier.
- Rebase your feature branch with the latest changes from the main branch to stay up to date.
- Add tests for new features to ensure they work as expected.
- Update documentation if necessary.

## Code of Conduct

We are committed to fostering an open and welcoming environment. By participating, you are expected to uphold the [Code of Conduct](CODE_OF_CONDUCT.md).

## Queries

If you have any questions or need further clarification about contributing, feel free to open an issue for discussion or ask for help on a specific topic.

Thank you for your interest in contributing to OpenLIT!
