# goose Technical Governance and Stewardship

Learn about goose's governance structure and how to participate

goose follows a lightweight technical governance model designed to support rapid iteration while maintaining community involvement. This document outlines how the project is organized and how decisions are made.

## Core Values

goose's governance is guided by three fundamental values:

* **Open**: goose is open source, but we go beyond code availability. We plan and build in the open. Our roadmap as well as goose recipes, extensions, and prompts are editable and shareable. Our goal is to make goose the most hackable agent available.
* **Flexible**: we prefer open models – but we don’t restrict ourselves. goose equally supports remotely deployed frontier models as well as local private models, whether open or proprietary.
* **Choice**: We're not bound to any one model, protocol, or stack. goose is built for choice and open standards, adapting to your tools, workflow, and identity as a creator.

## Technical Governance

goose adopts a streamlined two-tier structure optimized for speed and flexibility:

* **Core Maintainers** drive overall project direction and make final decisions
* **Maintainers** who have demonstrated extraordinary contributions and help drive specific components

All Maintainers are expected to embody goose's philosophy of openness and user autonomy. Membership in the technical governance process is for individuals, not companies.

### Core Maintainers

Core Maintainers are members of the goose team responsible for:

* Setting the overall technical direction and vision for goose
* Reviewing and merging pull requests
* Making architectural decisions along with Maintainers
* Maintaining release processes
* Resolving disputes and contentious issues
* Appointing Maintainers
* Ensuring goose remains fast-moving and experimental
* Ensuring the quality and stability of goose

Core Maintainers have full write access to the goose repositories and infrastructure.

### Maintainers

Maintainers are exceptional contributors from the broader community who have:

* Demonstrated deep understanding of goose through significant contributions
* Shown alignment with goose's values and technical direction
* Consistently provided high-quality code reviews and community support

Maintainers can:

* Submit pull requests and push branches directly to the main repository
* Review and provide feedback on pull requests
* Help triage issues and guide contributors
* Participate in technical discussions and planning

Maintainers have write access for creating pull requests but cannot directly merge to main or modify sensitive repository settings.

## Decision Making

### Fast-Track Process

To maintain goose's experimental and fast-moving nature:

* Most decisions are made through informal consensus in pull requests, GitHub discussions, and issues  
* Core Maintainers can approve and merge changes quickly when there's clear benefit  
* Significant architectural changes ([such as adopting ACP](https://github.com/block/goose/discussions/4645)) should have discussion in a GitHub issue or discussion before implementation  
* We optimize for shipping and iterating rather than lengthy deliberation

### Community Input

While we move fast, we value community input:

* All changes happen through pull requests, providing visibility  
* Significant features should have an associated GitHub issue describing the feature and testing approach  
* Community feedback is actively sought on Discord and GitHub discussions  
* External contributions are reviewed within two days, with merge/close decisions within two weeks

## Working Practices

### Code Review

Following our way of working:

* **Review AI-generated work carefully**: Check for redundant comments, bloated tests, outdated patterns, and repeated code
* **Prioritize reviews**: Others are waiting, but take time to understand the changes
* **Avoid review shopping**: Seek review from those familiar with the code being modified
* **Test thoroughly**: Manual and automated E2E testing is essential for larger features; post videos for UI changes

### Contributing

* **Discuss first**: For new features or architectural changes, open an issue or discussion
* **Keep PRs focused**: Smaller, focused changes are easier to review and merge
* **Write meaningful tests**: Tests should guard against real bugs, not just increase coverage
* **Engage with the community**: All Maintainers should be active on Discord and on GitHub, and be responsive to other contributors

### Release Process

* Regular releases with clear documentation of delivered features
* Quick bug fixes or security resolutions are cherry-picked to patch releases when needed
* All releases are tested by multiple Core Maintainers or Maintainers before publication

## Communication

### Channels

* **GitHub**: Primary platform for PRs, issues, and technical discussions
* **Discord**: Real-time community discussion and support

### Transparency

* All technical decisions are made in public through GitHub and Discord
* Meeting notes and significant decisions are shared with the community on GitHub
* Roadmap and priorities are openly discussed and published on GitHub

## Nominating Maintainers

### Principles

* Recognition is based on individual merit and contributions
* No term limits, but inactive Maintainers may be moved to emeritus status
* Membership is for individuals, not their employers

### Process

1. Core Maintainers identify exceptional contributors through:
   - History of high-quality merged PRs
   - Consistent helpful code reviews
   - Strong community engagement
   - Alignment with goose values
2. Discussion among Core Maintainers about the nomination
3. If approved, the contributor is invited to become a Maintainer
4. Announcement made to the community via Discord and GitHub

### Removal

Core Maintainers may remove Maintainer status if:

* Extended inactivity (3+ months without contribution)
* Actions contrary to goose's values
* By request of the Maintainer
* Appeals can be sent to the Core Maintainers group with rationale on why someone disagrees with the removal decision, with data to back up their case. As long as there is new information and good reasons to reverse the decision the appeal will be considered.

## Current Membership

Core Maintainers and Maintainers are listed in the main goose repository's [MAINTAINERS.md](https://github.com/block/goose/blob/main/MAINTAINERS.md) file with their areas of expertise where applicable.

## Evolution of Governance

This governance model is designed to be lightweight and may evolve as goose grows. Changes to governance require:

1. Proposal via GitHub issue with rationale
2. Community discussion period (minimum 1 week)
3. Consensus among Core Maintainers
4. Clear communication of changes to the community
5. A PR to the [GOVERNANCE.md](https://github.com/block/goose/blob/main/GOVERNANCE.md) file in the main goose repository

The key principle is maintaining goose's ability to move fast and experiment while respecting community contributions and maintaining transparency.

## Summary

goose's governance prioritizes:

* **Speed**: Minimal process to support rapid experimentation
* **Openness**: Transparent decision-making and community involvement
* **Autonomy**: Empowering users and contributors to shape goose
* **Quality**: Thoughtful review while avoiding bureaucracy

We believe this balance enables goose to remain innovative while building a strong, engaged community around the shared goal of creating the most hackable, user-controlled AI agent available.
