#!/usr/bin/env node

import { startStdioMcpServer } from "../_shared/mcp-stdio.mjs";
import {
  analyzeObservable,
  collectSuspiciousTokens,
  extractObservables,
} from "../_shared/security-observables.mjs";

const DEFAULT_FETCH_TIMEOUT_MS = 10_000;
const DEFAULT_MAX_HTML_BYTES = 200_000;

function stripTags(value) {
  return value.replace(/<script[\s\S]*?<\/script>/gi, " ").replace(/<style[\s\S]*?<\/style>/gi, " ").replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim();
}

function extractFirstMatch(pattern, source) {
  const match = source.match(pattern);
  return match?.[1]?.replace(/\s+/g, " ").trim() ?? "";
}

function extractLinks(html, baseUrl) {
  const links = [];
  const pattern = /<a\b[^>]*href=["']?([^"' >]+)[^>]*>([\s\S]*?)<\/a>/gi;

  for (const match of html.matchAll(pattern)) {
    const rawHref = match[1]?.trim();
    if (!rawHref) {
      continue;
    }

    let resolvedHref = rawHref;
    let host = "";
    try {
      const parsed = baseUrl ? new URL(rawHref, baseUrl) : new URL(rawHref);
      resolvedHref = parsed.toString();
      host = parsed.hostname;
    } catch {}

    links.push({
      href: resolvedHref,
      text: stripTags(match[2] ?? "").slice(0, 120),
      host,
    });
  }

  return links;
}

function extractForms(html, baseUrl) {
  const forms = [];
  const pattern = /<form\b([^>]*)>([\s\S]*?)<\/form>/gi;

  for (const match of html.matchAll(pattern)) {
    const attrs = match[1] ?? "";
    const body = match[2] ?? "";
    const action = extractFirstMatch(/\baction=["']?([^"' >]+)/i, attrs);
    const method = extractFirstMatch(/\bmethod=["']?([^"' >]+)/i, attrs).toUpperCase() || "GET";
    const inputNames = Array.from(
      body.matchAll(/<(input|textarea|select)\b[^>]*name=["']?([^"' >]+)/gi),
      (entry) => entry[2],
    );
    const suspicious = inputNames.some((name) =>
      ["password", "token", "otp", "mfa", "email"].includes(name.toLowerCase()),
    );

    forms.push({
      action: action
        ? (() => {
            try {
              return baseUrl ? new URL(action, baseUrl).toString() : action;
            } catch {
              return action;
            }
          })()
        : "",
      method,
      inputNames,
      suspicious,
    });
  }

  return forms;
}

function extractScripts(html, baseUrl) {
  const external = [];
  let inlineCount = 0;

  for (const match of html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)) {
    const attrs = match[1] ?? "";
    const src = extractFirstMatch(/\bsrc=["']?([^"' >]+)/i, attrs);
    if (src) {
      try {
        external.push(baseUrl ? new URL(src, baseUrl).toString() : src);
      } catch {
        external.push(src);
      }
      continue;
    }
    if ((match[2] ?? "").trim()) {
      inlineCount += 1;
    }
  }

  return {
    external,
    inlineCount,
  };
}

async function loadHtmlFromArgs(args) {
  if (typeof args.html === "string" && args.html.trim()) {
    return {
      source: "inline-html",
      requestedUrl: typeof args.source_url === "string" ? args.source_url : undefined,
      finalUrl: typeof args.source_url === "string" ? args.source_url : undefined,
      status: null,
      html: args.html.slice(0, Number(args.maxBytes) || DEFAULT_MAX_HTML_BYTES),
    };
  }

  if (typeof args.url !== "string" || !args.url.trim()) {
    throw new Error("Expected either html or url input");
  }

  const url = new URL(args.url);
  if (!["http:", "https:"].includes(url.protocol)) {
    throw new Error("Only http/https URLs are supported in local preview mode");
  }

  const response = await fetch(url, {
    redirect: "follow",
    signal: AbortSignal.timeout(Number(args.timeoutMs) || DEFAULT_FETCH_TIMEOUT_MS),
    headers: {
      "user-agent": "security-goose-browser-assist/0.1",
      accept: "text/html,application/xhtml+xml",
    },
  });
  const html = (await response.text()).slice(0, Number(args.maxBytes) || DEFAULT_MAX_HTML_BYTES);

  return {
    source: "live-fetch",
    requestedUrl: args.url,
    finalUrl: response.url,
    status: response.status,
    html,
  };
}

async function summarizePage(args) {
  const payload = await loadHtmlFromArgs(args);
  const baseUrl = payload.finalUrl ?? payload.requestedUrl;
  const html = payload.html;
  const text = stripTags(html);
  const title = extractFirstMatch(/<title[^>]*>([\s\S]*?)<\/title>/i, html);
  const metaDescription = extractFirstMatch(
    /<meta\b[^>]*name=["']description["'][^>]*content=["']([^"']*)/i,
    html,
  );
  const metaRobots = extractFirstMatch(
    /<meta\b[^>]*name=["']robots["'][^>]*content=["']([^"']*)/i,
    html,
  );
  const links = extractLinks(html, baseUrl).slice(0, 20);
  const forms = extractForms(html, baseUrl);
  const scripts = extractScripts(html, baseUrl);
  const observables = extractObservables(`${html}\n${baseUrl ?? ""}`);
  const suspiciousTokens = collectSuspiciousTokens(`${title} ${text}`.slice(0, 10_000));

  return {
    previewMode: "local-read-only",
    source: payload.source,
    requestedUrl: payload.requestedUrl,
    finalUrl: payload.finalUrl,
    httpStatus: payload.status,
    title,
    metaDescription,
    metaRobots,
    textSample: text.slice(0, 500),
    suspiciousTokens,
    counts: {
      links: links.length,
      forms: forms.length,
      externalScripts: scripts.external.length,
      inlineScripts: scripts.inlineCount,
    },
    forms,
    links,
    externalScripts: scripts.external.slice(0, 15),
    observables,
  };
}

async function extractPageObservables(args) {
  const payload = await loadHtmlFromArgs(args);
  const baseUrl = payload.finalUrl ?? payload.requestedUrl;
  const html = payload.html;
  const observables = extractObservables(`${html}\n${baseUrl ?? ""}`);
  const analyzed = [
    ...(observables.urls ?? []).slice(0, 5).map((entry) => analyzeObservable(entry)),
    ...(observables.domains ?? []).slice(0, 5).map((entry) => analyzeObservable(entry)),
    ...(observables.ipv4 ?? []).slice(0, 5).map((entry) => analyzeObservable(entry)),
  ];

  return {
    previewMode: "local-read-only",
    source: payload.source,
    requestedUrl: payload.requestedUrl,
    finalUrl: payload.finalUrl,
    observables,
    analyzedSample: analyzed,
  };
}

startStdioMcpServer({
  name: "browser-assist-mcp",
  version: "0.1.0",
  instructions:
    "Read-only browser-assist preview for Goose. It fetches static HTML over http/https or analyzes inline HTML, then extracts observable clues for web investigation. It does not execute JavaScript, log in, or mutate targets.",
  tools: [
    {
      name: "summarize_web_page",
      description: "Fetch or inspect a web page snapshot and summarize forms, links, scripts, and suspicious page signals.",
      inputSchema: {
        type: "object",
        properties: {
          url: { type: "string", description: "http/https URL to fetch in read-only preview mode." },
          html: { type: "string", description: "Optional inline HTML to inspect without network access." },
          source_url: { type: "string", description: "Optional original URL when html is passed inline." },
          timeoutMs: { type: "number", minimum: 1000, maximum: 30000 },
          maxBytes: { type: "number", minimum: 1024, maximum: 500000 },
        },
      },
      handler: summarizePage,
    },
    {
      name: "extract_page_observables",
      description: "Extract URLs, domains, IPs, emails, and related observables from a fetched or inline page snapshot.",
      inputSchema: {
        type: "object",
        properties: {
          url: { type: "string" },
          html: { type: "string" },
          source_url: { type: "string" },
          timeoutMs: { type: "number", minimum: 1000, maximum: 30000 },
          maxBytes: { type: "number", minimum: 1024, maximum: 500000 },
        },
      },
      handler: extractPageObservables,
    },
  ],
});
