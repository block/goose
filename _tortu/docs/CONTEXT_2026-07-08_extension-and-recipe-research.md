# Context: Goose extension & recipe research

**From:** Cowork (genbu-dev thread, moved here per Doug 2026-07-08) **Status:** reference/context only — no action taken yet.

## Why this exists

Doug went looking on Goose's Discord for a "getting started" guide — what recipes to install, what
extensions to add to a vanilla install — and couldn't find one. Asked for a search. Answer: it's not
on Discord at all. It's on the docs site.

## Where the real resources live

`goose-docs.ai` has a "Resources" menu (top nav) that Discord doesn't surface at all:

- [Extensions catalog](https://goose-docs.ai/extensions) — full list, 66 entries as of this research
- [Skills Marketplace](https://goose-docs.ai/skills)
- [Recipe Cookbook](https://goose-docs.ai/recipes) — official curated recipes
- [Recipe Generator](https://goose-docs.ai/recipe-generator)
- [Prompt Library](https://goose-docs.ai/prompt-library)
- [Deeplink Generator](https://goose-docs.ai/deeplink-generator)
- [Goose Subagents Guide](https://block.github.io/goose/docs/guides/subagents/)
- [Recipe Reference](https://block.github.io/goose/docs/guides/recipes/recipe-reference)

Also worth noting: Goose moved to the Agentic AI Foundation (AAIF) as of April 2026 —
`https://goose-docs.ai/blog/2026/04/07/goose-moves-to-aaif`. Repo is `github.com/aaif-goose/goose`
now, not `block/goose`.

## Extensions worth adding to a vanilla install

Picked from the full 66-extension catalog (below) for Doug's actual use — heavy planning/research/build
work, not a generic dev setup:

- **Memory / Knowledge Graph Memory / Cognee** — three flavors of persistent memory as installable
  extensions. Worth trying Cognee as a quick baseline before wiring anything custom (Letta/Mem0-style).
- **Chat Recall** — search conversation history and load session summaries across all Goose sessions.
  Directly useful for cross-session continuity.
- **Skills + Summon** — Skills loads reusable instruction sets; Summon loads skills *and* delegates to
  subagents. This is Goose's native mechanism for a skills-library pattern — worth testing hands-on.
- **Extension Manager** — dynamically enable/disable extensions mid-session instead of reconfiguring.
  Keeps a lean default set, pulls in specialty tools only when needed.
- **Todo + Top Of Mind** — Todo breaks work into trackable steps; Top Of Mind injects persistent
  instructions into working memory every turn.
- **Tavily / Exa / Firecrawl / Fetch / Context7 / GitMCP / goose Docs** — the research stack. Context7
  and GitMCP specifically pull live docs/repo content into context rather than relying on training data.
- **GitHub** — direct GitHub operations as an extension.
- **Container Use** — isolated dev environments per task, worth having before letting Goose run
  build/test loops unsupervised.
- **Code Mode** — lets Goose write JS to drive multiple MCP tools in one shot instead of one-tool-call-
  at-a-time. Real efficiency gain once several extensions are running together.
- **Computer Controller** — the one used in Goose's own quickstart tutorial; browser/webscraping/file
  automation.
- **Council of Mine** — multi-perspective consultation extension.

## Full extension catalog (for reference — 66 entries, alphabetical)

AgentQL · Alby Bitcoin Payments · Apify · Apps · Asana · Auto Visualiser · Beads · Blender · Browserbase ·
Cash App · Chat Recall · Chrome DevTools · Cloudinary Asset Management · Code Mode · Cognee ·
Computer Controller · Container Use · Context7 · Council of Mine · DataHub · Dev.to · Developer ·
ElevenLabs · Exa Search · Excalidraw · Extension Manager · Fetch · Figma · Firecrawl · GitHub · GitMCP ·
goose Docs · gotoHuman · I Ching · JetBrains · Knowledge Graph Memory · Linux MCP Server · mbot ·
Memory · MongoDB · Nano Banana · Neighborhood · Neon · Netlify · Nostrbook · OpenMetadata · PDF Reader ·
Pieces for Developers · Playwright · prompts.chat · Reddit · Rendex · Repomix · Rube · Selenium ·
Skills · Square MCP · Sugar · Summon · Supabase · Tavily Web Search · Todo · Top Of Mind · Tutorial ·
Vercel · VMware AIops · YouTube Transcript

Full catalog with per-extension install docs: `https://goose-docs.ai/docs/category/mcp-servers`
(each entry links to `https://goose-docs.ai/docs/mcp/<name>-mcp`).

## Recipes

**Community starter pack:** [Goose Subagents Recipe Collection](https://gist.github.com/mootrichard/8a2e4bf200e750f54bebfc78bbe4601f)
— actively maintained, from Block's own "Goose Subagents Workshop"
(`https://github.com/block/goose-subagents-workshop`). Six ready-to-use recipes:

1. **Code Review Assistant** (`code-reviewer`) — quality/security/performance/readability review.
   Param: `focus_area`. Gist: `gist.github.com/mootrichard/5a31b9cc64b77f1ae55e502daca4d4a0`
2. **Test Generator** (`test-generator`) — comprehensive test suites, happy path + edge cases + errors.
   Params: `test_framework`, `coverage_level`. Gist: `gist.github.com/mootrichard/0a78f04c38930936864595d92e23dd57`
3. **Documentation Writer** (`doc-writer`) — API docs, guides, tutorials, README generation.
   Params: `doc_type`, `audience`. Gist: `gist.github.com/mootrichard/3ac5e9ead0ca120acaf7df8d542abbe9`
4. **Codebase Implementation Analyzer** (`codebase-analyzer`) — deep implementation analysis, data flow
   tracing, file:line references. Params: `focus_area`, `component`.
   Gist: `gist.github.com/mootrichard/9628d286a5c15154535357e3ceb0e40f`
5. **Codebase File Locator** (`codebase-locator`) — finds all files related to a feature.
   Params: `feature` (required), `file_types`. Gist: `gist.github.com/mootrichard/8399b904ac301c643968cc8835163c35`
6. **Web Research Specialist** (`web-researcher`) — current docs/best-practices/solutions with citations.
   Params: `topic` (required), `source_type`. Gist: `gist.github.com/mootrichard/d7840a2d498fcdad67656c25dc9e88a3`

Install pattern (per the gist):

```
curl -o code-reviewer.yaml https://gist.githubusercontent.com/mootrichard/5a31b9cc64b77f1ae55e502daca4d4a0/raw/code-reviewer.yaml
# ...repeat per recipe...
mkdir -p ~/.goose/recipes
mv *.yaml ~/.goose/recipes/
export GOOSE_RECIPE_PATH=~/.goose/recipes
```

Documented workflow patterns worth keeping in mind when combining these: a parallel feature-dev
pipeline (locator + researcher + analyzer in parallel, then generator + doc-writer), a three-angle
code-quality audit (code-reviewer run three times with different `focus_area`), a bug-investigation
chain (analyzer → locator → generator), and a documentation sprint (analyzer + doc-writer per
component, in parallel across components).

**Doug's actual recipe directory:** `~/AOF/resources/goose/recipes/` (not `~/.goose/recipes/` — that
was the source of the duplicate-listing mess that led to the `recipe list` formatting patch already
staged in `../patches/recipe-list-formatting/`). Any newly downloaded recipes should go there, and
`GOOSE_RECIPE_PATH` should point only at that one directory to avoid recreating the duplication.

## Suggested next actions (not yet done — for whoever picks this up)

- Review the recommended extension list above against what's already installed; install the ones that
  fit.
- Consider fetching the six `mootrichard` recipes into this repo's `recipes/community/` folder (already
  exists, currently empty) so they're version-controlled here rather than living only in
  `~/AOF/resources/goose/recipes/`.
- Cross-reference: the `recipe list` formatting patch already staged in `../patches/recipe-list-
  formatting/` is a related but separate piece of work — that one's about fixing native CLI output,
  this note is about what to actually install.
