// Holds URL-supplied recipe parameters for a warm-launch deep link. The ACP
// parameter request fires mid-`session/new`, before a session id exists, so the
// caller stashes the values here and takePendingRecipeParameters consumes them once.
let pendingRecipeParameters: Record<string, string> | undefined;

export function setPendingRecipeParameters(parameters: Record<string, string> | undefined): void {
  pendingRecipeParameters =
    parameters && Object.keys(parameters).length > 0 ? parameters : undefined;
}

export function takePendingRecipeParameters(): Record<string, string> | undefined {
  const value = pendingRecipeParameters;
  pendingRecipeParameters = undefined;
  return value;
}
