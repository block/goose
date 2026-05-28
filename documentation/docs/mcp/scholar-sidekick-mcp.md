---
title: Scholar Sidekick Extension
description: Add Scholar Sidekick MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Scholar Sidekick MCP Server](https://github.com/mlava/scholar-sidekick-mcp) as a goose extension to resolve, format, export, and verify academic citations from any scholarly identifier — DOI, PMID, PMCID, ISBN, ISSN, arXiv ID, ADS bibcode, or WHO IRIS URL — plus retraction (Crossref + Retraction Watch) and open-access (Unpaywall) checks.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=scholar-sidekick-mcp%40latest&id=scholar-sidekick&name=Scholar%20Sidekick&description=Resolve%2C%20format%2C%20export%2C%20and%20verify%20academic%20citations%20plus%20retraction%20and%20open-access%20checks&env=RAPIDAPI_KEY%3DRapidAPI%20key%20for%20the%20Scholar%20Sidekick%20API%20%28free%20tier%20available%29&timeout=300)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y scholar-sidekick-mcp@latest
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  RAPIDAPI_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`. You'll also need a free [RapidAPI key for Scholar Sidekick](https://rapidapi.com/scholar-sidekick-scholar-sidekick-api/api/scholar-sidekick) — the free tier is enough for most personal use.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="scholar-sidekick"
      extensionName="Scholar Sidekick"
      description="Resolve, format, export, and verify academic citations plus retraction and open-access checks."
      type="stdio"
      command="npx"
      args={["-y", "scholar-sidekick-mcp@latest"]}
      timeout={300}
      envVars={[
        { name: "RAPIDAPI_KEY", label: "RapidAPI key for the Scholar Sidekick API" }
      ]}
      apiKeyLink="https://rapidapi.com/scholar-sidekick-scholar-sidekick-api/api/scholar-sidekick"
      apiKeyLinkText="RapidAPI Key"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Scholar Sidekick"
      description="Resolve, format, export, and verify academic citations plus retraction and open-access checks."
      type="stdio"
      command="npx -y scholar-sidekick-mcp@latest"
      timeout={300}
      envVars={[
        { key: "RAPIDAPI_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          Get your API key from{" "}
          <a href="https://rapidapi.com/scholar-sidekick-scholar-sidekick-api/api/scholar-sidekick" target="_blank" rel="noopener noreferrer">
            RapidAPI
          </a> and paste it in.
        </>
      }
    />
  </TabItem>
</Tabs>

## What You Can Do

Scholar Sidekick exposes six tools that turn any scholarly identifier into clean citations, exports, and integrity checks. Five built-in citation styles (Vancouver, AMA, APA, IEEE, CSE) plus the full CSL catalogue of 10,000+ styles.

### Format a citation from any identifier

Paste a DOI, PMID, PMCID, ISBN, ISSN, arXiv ID, ADS bibcode, or WHO IRIS URL and get a formatted reference in your chosen style. Detection is automatic — pass identifiers verbatim (no need to strip `PMID:`, `arXiv:`, or `https://doi.org/` prefixes).

**Prompt:**

```
Format 10.1056/NEJMoa2033700 as a Vancouver-style citation.
```

### Export a bibliography for your reference manager

Pass one or more identifiers and get a ready-to-import file in BibTeX, RIS, EndNote XML, RefWorks, NBIB, Zotero RDF, CSV, or CSL-JSON. Drop straight into Zotero, Mendeley, EndNote, JabRef, or Citavi.

**Prompt:**

```
Take these three DOIs and export them as BibTeX:
10.1056/NEJMoa2033700
10.1038/s41586-020-2649-2
10.1016/S0140-6736(20)32661-1
```

### Check whether a paper has been retracted

Cross-references Crossref and Retraction Watch for retractions, corrections, and expressions of concern. Returns status, reason, and date.

**Prompt:**

```
Has 10.1016/S0140-6736(97)11096-0 been retracted?
```

### Find an open-access copy

Looks up Unpaywall to surface the best legal free version of a paper — repository copy, publisher OA, or preprint — with the licence and version (accepted vs published).

**Prompt:**

```
Is there a free open-access copy of 10.1371/journal.pone.0173664?
```

### Verify whether a citation is real

Cross-checks a *claimed* citation (title, optional authors/year) against the metadata actually resolved from its identifier. Catches the dominant LLM-fabrication pattern — a real, resolvable DOI paired with an invented title and authors — documented by Topaz et al. (Lancet, 2026). Use this when an AI-generated bibliography "looks plausible but…".

**Prompt:**

```
Is this citation real? "A Unified Theory of Everything", Smith J, Nature, 2010, 10.1038/nphys1170
```
