#!/usr/bin/env node

import { startStdioMcpServer } from "../_shared/mcp-stdio.mjs";
import {
  analyzeObservable,
  detectObservableType,
  extractObservables,
  resolveDomainDns,
} from "../_shared/security-observables.mjs";

function ensureText(value) {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error("Expected a non-empty text field");
  }
  return value;
}

async function extractObservablesFromText(args) {
  const text = ensureText(args.text);
  const observables = extractObservables(text);

  return {
    previewMode: "local-read-only",
    counts: Object.fromEntries(
      Object.entries(observables).map(([key, value]) => [key, Array.isArray(value) ? value.length : 0]),
    ),
    observables,
  };
}

async function analyzeSingleObservable(args) {
  const observable = ensureText(args.observable);
  const explicitType =
    typeof args.type === "string" && args.type.trim() ? args.type.trim() : undefined;
  const analysis = analyzeObservable(observable, explicitType);

  return {
    previewMode: "local-read-only",
    analysis,
  };
}

async function enrichDomainDns(args) {
  const domain = ensureText(args.domain);
  if (detectObservableType(domain) !== "domain") {
    throw new Error("domain must be a valid hostname for DNS enrichment");
  }

  return {
    previewMode: "local-read-only",
    enrichment: await resolveDomainDns(domain),
  };
}

startStdioMcpServer({
  name: "threat-intel-mcp",
  version: "0.1.0",
  instructions:
    "Read-only local threat-intel preview for Goose. It extracts observables, performs heuristic classification, and can do unauthenticated DNS enrichment. It is not a vendor-backed threat feed and should not be presented as one.",
  tools: [
    {
      name: "extract_observables_from_text",
      description: "Extract URLs, domains, IPs, emails, and common hashes from a free-form text block.",
      inputSchema: {
        type: "object",
        required: ["text"],
        properties: {
          text: { type: "string", description: "Paste alert text, case notes, web content, or IOC batches." },
        },
      },
      handler: extractObservablesFromText,
    },
    {
      name: "analyze_observable",
      description: "Classify and normalize one observable with local heuristics suitable for IOC triage preview.",
      inputSchema: {
        type: "object",
        required: ["observable"],
        properties: {
          observable: { type: "string" },
          type: {
            type: "string",
            description: "Optional explicit type override such as url, domain, ipv4, email, md5, sha1, sha256.",
          },
        },
      },
      handler: analyzeSingleObservable,
    },
    {
      name: "enrich_domain_dns",
      description: "Resolve A/AAAA/MX/NS/CNAME/TXT records for a domain using local DNS without external API keys.",
      inputSchema: {
        type: "object",
        required: ["domain"],
        properties: {
          domain: { type: "string" },
        },
      },
      handler: enrichDomainDns,
    },
  ],
});
