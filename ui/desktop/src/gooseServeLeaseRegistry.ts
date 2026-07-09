import type { GooseServeExitSignal, GooseServeResult, Logger } from './gooseServe';

export const GOOSE_SERVE_EXITED_USER_MESSAGE =
  "This window's Goose backend stopped. Close this window and open a new chat to start a new backend. If this keeps happening, restart Goose Desktop.";

export interface GooseServeLease {
  acpUrl: string;
  secretKey: string;
  cleanup: () => Promise<void>;
  windowIds: Set<number>;
  cleanedUp: boolean;
  exited: boolean;
  exitCode: number | null;
  exitSignal: GooseServeExitSignal;
}

export class GooseServeLeaseRegistry {
  private leasesByWindowId = new Map<number, GooseServeLease>();
  private pendingCleanups = new Set<Promise<void>>();

  constructor(private readonly logger: Logger) {}

  create(result: GooseServeResult, secretKey: string): GooseServeLease {
    const lease: GooseServeLease = {
      acpUrl: result.acpUrl,
      secretKey,
      cleanup: result.cleanup,
      windowIds: new Set<number>(),
      cleanedUp: false,
      exited: false,
      exitCode: null,
      exitSignal: null,
    };

    const markExited = ({
      code,
      signal,
      logUnexpected,
    }: {
      code?: number | null;
      signal?: GooseServeExitSignal;
      logUnexpected: boolean;
    }) => {
      const firstExit = !lease.exited;
      lease.exited = true;
      if (code !== undefined) {
        lease.exitCode = code;
      }
      if (signal !== undefined) {
        lease.exitSignal = signal;
      }

      if (logUnexpected && firstExit && !lease.cleanedUp) {
        this.logger.error('Goose ACP server exited unexpectedly', {
          code: lease.exitCode,
          signal: lease.exitSignal,
          windowIds: [...lease.windowIds],
        });
      }
    };

    result.process.once('exit', (code, signal) => {
      markExited({ code, signal, logUnexpected: true });
    });

    if (result.hasExited()) {
      const exitDetails = result.getExitDetails();
      markExited({ code: exitDetails.code, signal: exitDetails.signal, logUnexpected: false });
    }

    return lease;
  }

  createExternal(
    acpUrl: string,
    secretKey: string,
    cleanup: () => Promise<void> = async () => undefined
  ): GooseServeLease {
    return {
      acpUrl,
      secretKey,
      cleanup,
      windowIds: new Set<number>(),
      cleanedUp: false,
      exited: false,
      exitCode: null,
      exitSignal: null,
    };
  }

  get(windowId: number): GooseServeLease | null {
    return this.leasesByWindowId.get(windowId) ?? null;
  }

  getAcpUrl(windowId: number): string | null {
    const lease = this.get(windowId);
    if (!lease) {
      return null;
    }
    if (lease.exited) {
      throw new Error(GOOSE_SERVE_EXITED_USER_MESSAGE);
    }
    return lease.acpUrl;
  }

  getSecretKey(windowId: number): string | null {
    const lease = this.get(windowId);
    if (!lease) {
      return null;
    }
    if (lease.exited) {
      throw new Error(GOOSE_SERVE_EXITED_USER_MESSAGE);
    }
    return lease.secretKey;
  }

  attachWindow(windowId: number, lease: GooseServeLease) {
    lease.windowIds.add(windowId);
    this.leasesByWindowId.set(windowId, lease);
  }

  async releaseWindow(windowId: number) {
    const lease = this.leasesByWindowId.get(windowId);
    this.leasesByWindowId.delete(windowId);

    if (!lease) {
      return;
    }

    lease.windowIds.delete(windowId);
    if (lease.windowIds.size === 0) {
      await this.cleanupLease(lease);
    }
  }

  async cleanupLease(lease: GooseServeLease) {
    if (lease.cleanedUp) {
      return;
    }

    lease.cleanedUp = true;
    for (const windowId of lease.windowIds) {
      this.leasesByWindowId.delete(windowId);
    }
    lease.windowIds.clear();

    // Track the in-flight cleanup so shutdown can await it. releaseWindow() is
    // invoked fire-and-forget on window close, and it removes the lease from the
    // registry before this async cleanup finishes. Without this tracking, a quit
    // that races the cleanup sees activeLeaseCount() === 0, skips cleanupAll(),
    // and exits while goosed is still being terminated -- orphaning the process.
    const task = (async () => {
      try {
        await lease.cleanup();
      } catch (error) {
        this.logger.error('Failed to cleanup goose serve backend:', error);
      }
    })();
    this.pendingCleanups.add(task);
    try {
      await task;
    } finally {
      this.pendingCleanups.delete(task);
    }
  }

  activeLeaseCount(): number {
    return this.uniqueLeases().length;
  }

  async cleanupAll() {
    await Promise.all(this.uniqueLeases().map((lease) => this.cleanupLease(lease)));
    await this.settlePendingCleanups();
  }

  /**
   * Whether any lease cleanup started fire-and-forget (e.g. by releaseWindow()
   * on window close) is still running. Lets the quit path decide whether it must
   * hold the app open until backend termination finishes.
   */
  hasPendingCleanups(): boolean {
    return this.pendingCleanups.size > 0;
  }

  /**
   * Await any in-flight lease cleanups started fire-and-forget (e.g. by
   * releaseWindow() on window close). Call this during app shutdown so the
   * process does not exit and orphan a goosed backend that is still being
   * terminated.
   */
  async settlePendingCleanups(): Promise<void> {
    while (this.pendingCleanups.size > 0) {
      await Promise.all([...this.pendingCleanups]);
    }
  }

  private uniqueLeases(): GooseServeLease[] {
    return [...new Set(this.leasesByWindowId.values())];
  }
}
