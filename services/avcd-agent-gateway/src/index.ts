/**
 * avcd-agent-gateway entrypoint.
 * Phase 0 scaffold — real HTTP/WS server lands in Phase 5.
 */
export function gatewayPlaceholder(): string {
  return 'avcd-agent-gateway-not-ready'
}

if (import.meta.url === `file://${process.argv[1]}`) {
  console.error(gatewayPlaceholder())
  process.exit(1)
}
