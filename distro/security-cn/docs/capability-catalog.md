# security-cn capability catalog

## Prompts source

- `system-zh.md`
- `system-en.md`
- `security-role-defaults.md`

## Skills source

These directories are the release-source copies. Goal 5 mirrors them to project
runtime under `.agents/skills/` for local Goose preview:

- `vuln-triage`
- `alert-triage`
- `ioc-analysis`
- `asset-risk-summary`
- `report-writing`
- `wooyun-legacy`
  - Goose-native wrapper skill for WooYun-style business-logic investigation
  - Optional upstream reference pack is local-preview only and not redistributed in this fork because of upstream `CC BY-NC-SA 4.0`

## Recipes source

These files remain the release-source copies. Goal 5 mirrors them to
`.goose/recipes/*.yaml` so Goose can discover them without a custom loader:

- `security-vuln-triage`
- `alert-investigation`
- `web-investigation`

## Goal 6 desktop entry mapping

The desktop starter entries remain a thin mapping layer on top of existing
Goose-native capabilities:

| Desktop task | Primary mapping | Secondary mapping | Runtime mode |
| --- | --- | --- | --- |
| 漏洞研判 | `security-vuln-triage` recipe | `vuln-triage` skill | recipe-backed |
| 告警分析 | `alert-investigation` recipe | `alert-triage` skill | recipe-backed |
| IOC 研判 | none | `ioc-analysis` skill | guided preview chat |
| 网页调查 | `web-investigation` recipe | `ioc-analysis` skill | recipe-backed |
| 报告生成 | none | `report-writing` skill | guided preview chat |
| 业务逻辑排查 | none | `wooyun-legacy` skill | guided preview chat |

Current desktop entry surfaces:

- Quick launcher overlay: `ui/desktop/src/components/LauncherView.tsx`
- Recipes page starter cards: `ui/desktop/src/components/recipes/RecipesView.tsx`

## Goal 9 task-to-extension recommendation

These recommendations stay optional. The tasks still launch through the existing
Goose recipe/skill path even when an extension remains off, stubbed, or blocked.

| Desktop task | Recommended extension | Current status |
| --- | --- | --- |
| `漏洞研判` | `aiseesec-mcp` | blocked by external dependency |
| `告警分析` | `threat-intel-mcp` | real local preview |
| `告警分析` | `local-security-gateway-mcp` | disabled stub |
| `IOC 研判` | `threat-intel-mcp` | real local preview |
| `网页调查` | `browser-assist-mcp` | real local preview |
| `网页调查` | `threat-intel-mcp` | real local preview |
| `业务逻辑排查` | `browser-assist-mcp` | real local preview |

## Extensions source

These entries sync into
`ui/desktop/src/components/settings/extensions/bundled-extensions.json`.

Goal 8 runtime split:

- real local preview
  - `threat-intel-mcp`
    - local IOC extraction
    - heuristic observable analysis
    - DNS enrichment without external API keys
  - `browser-assist-mcp`
    - static page fetch / inline HTML inspection
    - page summary, forms/links/scripts hints, observable extraction
- disabled stub / blocker
  - `aiseesec-mcp`
    - blocker: external proprietary API and account requirements
  - `local-security-gateway-mcp`
    - blocker: Goal 8 explicitly does not implement a real gateway
