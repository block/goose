let authFailureCount = 0

export function recordAuthFailure(): void {
  authFailureCount += 1
}

export function getAuthMetrics(): { authFailures: number } {
  return { authFailures: authFailureCount }
}

export function resetAuthMetricsForTests(): void {
  authFailureCount = 0
}
