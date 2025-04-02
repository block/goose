---
title: PDF Extension
description: Add Figma MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

<!-- <YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/VIDEO_ID" /> -->

This tutorial covers how to add the [PDF MCP Server](https://github.com/michaelneale/mcp-read-pdf) as a Goose extension, enabling Goose to read and extract text from protected and unprotected PDFs.

:::tip TLDR

**Command**

```sh
uvx mcp-read-pdf
```

:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`.
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

2. Choose to add a `Command-line Extension`

```sh
  ┌   goose-configure
  │
  ◇  What would you like to configure?
  │  Add Extension
  │
  ◆  What type of extension would you like to add?
  │  ○ Built-in Extension
  // highlight-start
  │  ● Command-line Extension (Run a local command or script)
  // highlight-end
  │  ○ Remote Extension
  └
```

3. Give your extension a name

```sh
  ┌   goose-configure
  │
  ◇  What would you like to configure?
  │  Add Extension
  │
  ◇  What type of extension would you like to add?
  │  Command-line Extension
  │
  // highlight-start
  ◆  What would you like to call this extension?
  │  pdf
  // highlight-end
  └
```

4. Enter the command

```sh
  ┌   goose-configure
  │
  ◇  What would you like to configure?
  │  Add Extension
  │
  ◇  What type of extension would you like to add?
  │  Command-line Extension
  │
  ◇  What would you like to call this extension?
  │  pdf
  │
  // highlight-start
  ◆  What command should be run?
  │  uvx mcp-read-pdf
  // highlight-end
  └
```

5. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s

   ```sh
   ┌   goose-configure
   │
   ◇  What would you like to configure?
   │  Add Extension
   │
   ◇  What type of extension would you like to add?
   │  Command-line Extension
   │
   ◇  What would you like to call this extension?
   │  pdf
   │
   ◇  What command should be run?
   │  uvx mcp-read-pdf
   │
   // highlight-start
   ◆  Please set the timeout for this tool (in secs):
   │  300
   // highlight-end
   │
   └
   ```

6. Choose No when asked to add environment variables

   ```sh
    ┌   goose-configure
    │
    ◇  What would you like to configure?
    │  Add Extension
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension
    │
    ◇  What would you like to call this extension?
    │  pdf
    │
    ◇  What command should be run?
    │  uvx mcp-read-pdf
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◆  Would you like to add environment variables?
    │  No
    │
    // highlight-end
    └  Added pdf extension
   ```

</TabItem>
<TabItem value="ui" label="Goose Desktop">
  1. [Launch the installer](goose://extension?cmd=uvx&arg=mcp-read-pdf&id=pdf_read&name=PDF%20Reader&description=Read%20large%20and%20complex%20PDF%20documents)
  2. Press `Yes` to confirm the installation
  3. Click `Save Configuration`
  4. Scroll to the top and click `Exit` from the upper left corner
</TabItem>
   </Tabs>

## Example Usage

This example shows how to use the PDF Extension to analyze an applicant's resume for specific attributes. With the PDF extension, Goose can read specific pages or entire documents as well as extract text from large and complex PDFs.

### Goose Prompt

```
Could you analyze the resume at ~/Downloads/resume.pdf focusing on:

1. Technical Expertise
   - Programming languages and proficiency levels
   - System design experience
   - Technical certifications

2. Project Implementation
   - Architecture decisions
   - Scale of systems built
   - Performance improvements achieved

3. Problem-Solving Capabilities
   - Technical challenges overcome
   - Innovation examples
   - Debug/troubleshooting experience
```

### Goose Output

:::note CLI

I'll help analyze that resume. Let me try to access the PDF file:

─── read_pdf | pdf ──────────────────

file_path: ~/Downloads/resume.pdf

**1. Technical Expertise**

**Programming Languages & Proficiency**

- **Advanced**: Python, Java, SQL
- **Intermediate**: JavaScript, C++
- **Notable**: Experience with both backend and frontend development

**System Design Experience**

- Microservices architecture implementation
- Distributed systems design
- Cloud infrastructure (AWS) expertise
- Database design and optimization

**Technical Certifications**

- AWS Certified Solutions Architect
- Professional Scrum Master (PSM I)
- MongoDB Certified Developer

**2. Project Implementation**

**Architecture Decisions**

- Led migration from monolithic to microservices architecture
- Implemented event-driven architecture for real-time data processing
- Designed scalable cloud-native solutions using AWS services

**Scale of Systems Built**

- Built systems handling:
  - 1M+ daily active users
  - 500+ transactions per second
  - Petabyte-scale data processing
- Managed distributed systems across multiple regions

**Performance Improvements**

- Reduced API response times by 60% through caching implementation
- Optimized database queries resulting in 40% faster data retrieval
- Implemented automated scaling solutions reducing infrastructure costs by 35%

**3. Problem-Solving Capabilities**

**Technical Challenges Overcome**

- Resolved critical production issues with minimal downtime
- Implemented robust error handling and recovery systems
- Developed solutions for complex data synchronization issues

**Innovation Examples**

- Created custom monitoring and alerting system
- Developed automated deployment pipeline reducing deployment time by 75%
- Pioneered machine learning integration for predictive system scaling

**Debug/Troubleshooting Experience**

- Led incident response team for critical production issues
- Implemented comprehensive logging and monitoring solutions
- Developed automated testing frameworks reducing bug detection time

**Key Strengths**

1. Strong foundation in both theoretical and practical aspects of software engineering
2. Proven track record of scaling systems and improving performance
3. Experience with modern development practices and tools

**Areas for Further Discussion**

1. Specific experiences with distributed systems challenges
2. Details about team leadership and mentoring experience
3. Views on emerging technologies and their potential impact

**Recommendations**

1. **Technical Deep Dive**: Discuss the microservices architecture implementation in detail
2. **Problem-Solving Assessment**: Explore specific examples of critical production issues resolved
3. **System Design Discussion**: Review approach to scalability and performance optimization

Would you like me to focus on any particular aspect of this analysis or provide more specific details about certain areas?

:::
