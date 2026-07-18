/**
 * Integrity verification for MCP App UI resources.
 *
 * When Goose renders an MCP App it fetches HTML from a `ui://` resource served
 * by the MCP server and runs it inside a sandboxed iframe. There is no
 * server-provided hash or signature to verify that HTML against, so a
 * compromised or malicious server can silently swap the UI a user already
 * trusts.
 *
 * As a first line of defense this module implements trust-on-first-use (TOFU)
 * integrity tracking: it records the SHA-256 of the HTML served for each
 * (extension, resource) pair the first time it is seen, and flags any later
 * fetch whose content differs. That surfaces tampering of a previously-seen
 * app UI. Persisted audit logging and a trusted-source allowlist (also part of
 * issue #8014) are intentionally left as follow-ups.
 */

export interface IntegrityCheckResult {
  /** Hex-encoded SHA-256 of the fetched HTML. */
  hash: string;
  /** True the first time this (extension, resource) pair is seen. */
  firstSeen: boolean;
  /** True when the content hash differs from the previously recorded one. */
  changed: boolean;
  /** The hash recorded on a prior fetch, when one exists. */
  previousHash?: string;
}

export async function computeResourceHash(content: string): Promise<string> {
  const bytes = new TextEncoder().encode(content);
  const digest = await globalThis.crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

export function resourceIntegrityKey(extensionName: string, resourceUri: string): string {
  // NUL separates the fields so distinct pairs cannot collide via concatenation.
  return `${extensionName}\u0000${resourceUri}`;
}

export class ResourceIntegrityTracker {
  private readonly hashes = new Map<string, string>();

  record(key: string, hash: string): IntegrityCheckResult {
    const previousHash = this.hashes.get(key);
    if (previousHash === undefined) {
      this.hashes.set(key, hash);
      return { hash, firstSeen: true, changed: false };
    }
    if (previousHash === hash) {
      return { hash, firstSeen: false, changed: false, previousHash };
    }
    this.hashes.set(key, hash);
    return { hash, firstSeen: false, changed: true, previousHash };
  }

  reset(): void {
    this.hashes.clear();
  }
}

export async function checkResourceIntegrity(
  tracker: ResourceIntegrityTracker,
  extensionName: string,
  resourceUri: string,
  html: string
): Promise<IntegrityCheckResult> {
  const hash = await computeResourceHash(html);
  return tracker.record(resourceIntegrityKey(extensionName, resourceUri), hash);
}
