import dns from "node:dns/promises";

const IPV4_REGEX =
  /\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b/g;
const URL_REGEX = /\bhttps?:\/\/[^\s<>"')\]]+/gi;
const DOMAIN_REGEX =
  /\b(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}\b/gi;
const EMAIL_REGEX = /\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,63}\b/gi;
const SHA256_REGEX = /\b[a-f0-9]{64}\b/gi;
const SHA1_REGEX = /\b[a-f0-9]{40}\b/gi;
const MD5_REGEX = /\b[a-f0-9]{32}\b/gi;
const SUSPICIOUS_TOKENS = [
  "login",
  "signin",
  "verify",
  "password",
  "reset",
  "admin",
  "wallet",
  "invoice",
  "urgent",
  "payment",
  "secure",
  "token",
  "mfa",
  "sso",
];

function unique(items) {
  return Array.from(new Set(items.filter(Boolean)));
}

function toAsciiLower(value) {
  return value.trim().toLowerCase();
}

function normalizeDomain(value) {
  return value.replace(/\.+$/, "").trim().toLowerCase();
}

function isPublicIpv4(value) {
  return classifyIpv4(value).scope === "public";
}

export function detectObservableType(value) {
  const input = value.trim();
  if (!input) {
    return "unknown";
  }

  try {
    const url = new URL(input);
    if (url.protocol === "http:" || url.protocol === "https:") {
      return "url";
    }
  } catch {}

  if (EMAIL_REGEX.test(input)) {
    EMAIL_REGEX.lastIndex = 0;
    return "email";
  }
  EMAIL_REGEX.lastIndex = 0;

  if (IPV4_REGEX.test(input)) {
    IPV4_REGEX.lastIndex = 0;
    return "ipv4";
  }
  IPV4_REGEX.lastIndex = 0;

  if (SHA256_REGEX.test(input)) {
    SHA256_REGEX.lastIndex = 0;
    return "sha256";
  }
  SHA256_REGEX.lastIndex = 0;

  if (SHA1_REGEX.test(input)) {
    SHA1_REGEX.lastIndex = 0;
    return "sha1";
  }
  SHA1_REGEX.lastIndex = 0;

  if (MD5_REGEX.test(input)) {
    MD5_REGEX.lastIndex = 0;
    return "md5";
  }
  MD5_REGEX.lastIndex = 0;

  if (DOMAIN_REGEX.test(input)) {
    DOMAIN_REGEX.lastIndex = 0;
    return "domain";
  }
  DOMAIN_REGEX.lastIndex = 0;

  return "unknown";
}

export function extractObservables(text) {
  const urls = unique(Array.from(text.matchAll(URL_REGEX), (match) => match[0]));
  const emails = unique(Array.from(text.matchAll(EMAIL_REGEX), (match) => toAsciiLower(match[0])));
  const ipv4 = unique(Array.from(text.matchAll(IPV4_REGEX), (match) => match[0]));
  const sha256 = unique(Array.from(text.matchAll(SHA256_REGEX), (match) => toAsciiLower(match[0])));
  const sha1 = unique(Array.from(text.matchAll(SHA1_REGEX), (match) => toAsciiLower(match[0])));
  const md5 = unique(Array.from(text.matchAll(MD5_REGEX), (match) => toAsciiLower(match[0])));

  const urlHosts = new Set();
  for (const entry of urls) {
    try {
      urlHosts.add(normalizeDomain(new URL(entry).hostname));
    } catch {}
  }

  const emailDomains = new Set(emails.map((entry) => normalizeDomain(entry.split("@")[1] ?? "")));
  const domains = unique(
    Array.from(text.matchAll(DOMAIN_REGEX), (match) => normalizeDomain(match[0])).filter(
      (entry) => !urlHosts.has(entry) && !emailDomains.has(entry),
    ),
  );

  return {
    urls,
    domains,
    ipv4,
    emails,
    sha256,
    sha1,
    md5,
  };
}

export function collectSuspiciousTokens(text) {
  const lowered = text.toLowerCase();
  return SUSPICIOUS_TOKENS.filter((token) => lowered.includes(token));
}

export function classifyIpv4(ip) {
  const octets = ip.split(".").map((part) => Number.parseInt(part, 10));
  if (octets.length !== 4 || octets.some((part) => Number.isNaN(part) || part < 0 || part > 255)) {
    return { scope: "invalid" };
  }

  if (octets[0] === 10) {
    return { scope: "private", reason: "RFC1918 10.0.0.0/8" };
  }
  if (octets[0] === 172 && octets[1] >= 16 && octets[1] <= 31) {
    return { scope: "private", reason: "RFC1918 172.16.0.0/12" };
  }
  if (octets[0] === 192 && octets[1] === 168) {
    return { scope: "private", reason: "RFC1918 192.168.0.0/16" };
  }
  if (octets[0] === 127) {
    return { scope: "loopback", reason: "127.0.0.0/8" };
  }
  if (octets[0] === 169 && octets[1] === 254) {
    return { scope: "link_local", reason: "169.254.0.0/16" };
  }
  if (octets[0] >= 224) {
    return { scope: "special", reason: "multicast or reserved range" };
  }
  return { scope: "public" };
}

export function analyzeObservable(observable, explicitType) {
  const normalizedInput = observable.trim();
  const type = explicitType ?? detectObservableType(normalizedInput);
  const riskSignals = [];
  const notes = [];

  if (type === "url") {
    const parsed = new URL(normalizedInput);
    const hostType = detectObservableType(parsed.hostname);
    const tokens = collectSuspiciousTokens(
      `${parsed.hostname} ${parsed.pathname} ${parsed.search} ${parsed.hash}`,
    );

    if (parsed.username || parsed.password) {
      riskSignals.push("url-contains-userinfo");
    }
    if (hostType === "ipv4" && isPublicIpv4(parsed.hostname)) {
      riskSignals.push("url-uses-public-ip-host");
    }
    if (tokens.length > 0) {
      riskSignals.push("url-contains-suspicious-keywords");
    }

    return {
      observable: normalizedInput,
      type,
      normalized: parsed.toString(),
      host: parsed.hostname,
      hostType,
      path: parsed.pathname,
      queryKeys: unique(Array.from(parsed.searchParams.keys())),
      suspiciousTokens: tokens,
      riskSignals,
      notes,
    };
  }

  if (type === "domain") {
    const normalized = normalizeDomain(normalizedInput);
    const labels = normalized.split(".");
    const tokens = collectSuspiciousTokens(normalized);

    if (normalized.startsWith("xn--")) {
      riskSignals.push("punycode-domain");
    }
    if (tokens.length > 0) {
      riskSignals.push("domain-contains-suspicious-keywords");
    }

    return {
      observable: normalizedInput,
      type,
      normalized,
      tld: labels.at(-1) ?? "",
      labelCount: labels.length,
      suspiciousTokens: tokens,
      riskSignals,
      notes,
    };
  }

  if (type === "ipv4") {
    const scope = classifyIpv4(normalizedInput);
    if (scope.scope === "public") {
      notes.push("Public IPv4 should be cross-checked with ownership and exposure context.");
    }

    return {
      observable: normalizedInput,
      type,
      normalized: normalizedInput,
      scope,
      riskSignals,
      notes,
    };
  }

  if (type === "email") {
    const normalized = toAsciiLower(normalizedInput);
    const domain = normalizeDomain(normalized.split("@")[1] ?? "");
    const tokens = collectSuspiciousTokens(normalized);
    if (tokens.length > 0) {
      riskSignals.push("email-contains-suspicious-keywords");
    }

    return {
      observable: normalizedInput,
      type,
      normalized,
      domain,
      suspiciousTokens: tokens,
      riskSignals,
      notes,
    };
  }

  if (["md5", "sha1", "sha256"].includes(type)) {
    return {
      observable: normalizedInput,
      type,
      normalized: toAsciiLower(normalizedInput),
      riskSignals,
      notes: [
        "Hash-only preview does not prove maliciousness; pair it with sample source and execution context.",
      ],
    };
  }

  return {
    observable: normalizedInput,
    type: "unknown",
    normalized: normalizedInput,
    riskSignals,
    notes: ["Unsupported observable type for local preview enrichment."],
  };
}

export async function resolveDomainDns(domain) {
  const target = normalizeDomain(domain);
  const lookups = await Promise.allSettled([
    dns.resolve4(target),
    dns.resolve6(target),
    dns.resolveMx(target),
    dns.resolveNs(target),
    dns.resolveCname(target),
    dns.resolveTxt(target),
  ]);

  const [a, aaaa, mx, ns, cname, txt] = lookups;

  return {
    domain: target,
    a: a.status === "fulfilled" ? a.value : [],
    aaaa: aaaa.status === "fulfilled" ? aaaa.value : [],
    mx:
      mx.status === "fulfilled"
        ? mx.value.map((entry) => ({ exchange: entry.exchange, priority: entry.priority }))
        : [],
    ns: ns.status === "fulfilled" ? ns.value : [],
    cname: cname.status === "fulfilled" ? cname.value : [],
    txt:
      txt.status === "fulfilled"
        ? txt.value.map((entry) => entry.join(""))
        : [],
  };
}
