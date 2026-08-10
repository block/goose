export interface IntegrityCheckResult {
  hash: string;
  firstSeen: boolean;
  changed: boolean;
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

  constructor(private readonly maxEntries = 1000) {}

  record(key: string, hash: string): IntegrityCheckResult {
    const previousHash = this.hashes.get(key);
    if (previousHash === undefined) {
      if (this.hashes.size === this.maxEntries) {
        this.hashes.delete(this.hashes.keys().next().value!);
      }
      this.hashes.set(key, hash);
      return { hash, firstSeen: true, changed: false };
    }
    if (previousHash === hash) {
      return { hash, firstSeen: false, changed: false, previousHash };
    }
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
