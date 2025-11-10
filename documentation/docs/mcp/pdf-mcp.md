---
title: PDF Reader Extension
description: Add PDF Reader MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/EJf2_iZfaWk" />

This tutorial covers how to add the [PDF Reader MCP Server](https://github.com/michaelneale/mcp-read-pdf) as a goose extension, enabling goose to read and extract text from protected and unprotected PDFs.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=mcp-read-pdf&id=pdf_read&name=PDF%20Reader&description=Read%20large%20and%20complex%20PDF%20documents)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  uvx mcp-read-pdf
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="pdf_read"
    extensionName="PDF Reader"
    description="Read large and complex PDF documents"
    command="uvx"
    args={["mcp-read-pdf"]}
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="PDF Reader"
      description="Read large and complex PDF documents"
      command="uvx mcp-read-pdf"
    />
  </TabItem>
</Tabs>

## Example Usage

This example shows how to use the PDF Reader Extension to analyze an applicant's resume for specific attributes. With the PDF Reader extension, goose can read specific pages or entire documents as well as extract text from large and complex PDFs.

### goose Prompt

```
Read the resume at ~/Downloads/resume.pdf and evaluate how well this candidate aligns with the following role requirements:

  - 5+ years of backend development experience
  - Strong system design and distributed systems knowledge
  - Cloud infrastructure experience (AWS preferred)
  - Prior experience leading technical projects or teams
  - Bonus: familiarity with LLMs or AI/ML tools

Give a score out of 5 for each requirement, supporting evidence, and a summary at the end with pros, cons, and a decision: Strong Fit / Moderate Fit / Weak Fit.
```

### goose Output

:::note CLI

I'll help analyze the resume against those requirements. Let me read the PDF first.

─── read_pdf | pdf ──────────────────

file_path: ~/Downloads/resume.pdf

I'll now evaluate the candidate against each requirement based on the resume content:

**1. Backend Development Experience (5+ years)**

**Score: 5/5**

- 7+ years of experience as a Software Engineer at various companies
- Extensive backend development work with Java, Python, and Go
- Built and maintained multiple backend services and APIs
- Deep experience with microservices architecture and RESTful APIs

**2. System Design and Distributed Systems Knowledge**

**Score: 4.5/5**

- Designed and implemented distributed caching systems
- Experience with high-throughput message queuing systems
- Implemented service mesh architecture for microservices
- Strong understanding of scalability and reliability patterns
- Worked on systems processing millions of daily transactions

**3. Cloud Infrastructure Experience (AWS preferred)**

**Score: 4/5**

- Significant AWS experience including:
  - EC2, ECS, and Lambda
  - S3 and DynamoDB
  - CloudWatch and CloudFormation
- Experience with infrastructure as code
- Kubernetes deployment and management
- CI/CD pipeline implementation in cloud environments

**4. Prior Experience Leading Technical Projects or Teams**

**Score: 4.5/5**

- Led a team of 5 engineers in developing a new payment processing system
- Mentored junior developers and conducted technical interviews
- Managed multiple critical projects from inception to delivery
- Successfully coordinated cross-functional teams
- Demonstrated technical leadership in architecture decisions

**5. Bonus: Familiarity with LLMs or AI/ML Tools**

**Score: 3.5/5**

- Recent experience implementing ML model deployment pipelines
- Worked with natural language processing systems
- Basic understanding of ML concepts and frameworks
- Some exposure to LLM integration, though not extensive

**Summary**

**Pros:**

- Exceeds the required years of backend development experience
- Strong technical depth across multiple domains
- Proven leadership and project management capabilities
- Comprehensive cloud infrastructure experience
- Good foundation in modern AI/ML technologies

**Cons:**

- While experienced with AWS, some newer cloud services experience could be beneficial
- AI/ML experience is good but not deeply specialized in LLMs
- Could benefit from more experience with specific cloud-native patterns

**Decision: Strong Fit**

The candidate demonstrates exceptional strength in core requirements, particularly in backend development and system design. Their leadership experience and cloud infrastructure knowledge are very solid. While their AI/ML experience isn't extensive, it's sufficient for the role where it's listed as a bonus qualification. The breadth and depth of their technical experience, combined with demonstrated leadership capabilities, make them a strong fit for the position.

Their overall profile suggests they would be able to make immediate contributions while having the capacity to grow into any areas where they might need additional expertise. The strong foundation in distributed systems and proven track record of leading teams makes them particularly valuable for a technical leadership role.

**Overall Scores Summary:**

1. Backend Development: 5/5
2. System Design: 4.5/5
3. Cloud Infrastructure: 4/5
4. Leadership Experience: 4.5/5
5. AI/ML (Bonus): 3.5/5

**Total Score: 21.5/25** (not including bonus)

:::