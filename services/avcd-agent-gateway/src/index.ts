import { loadAuthSettings, logAuthSettingsOnStartup } from './auth/index.js'
import { InstanceSupervisor } from './instance/supervisor.js'
import { listenGateway } from './proxy/server.js'

export { createGatewayServer, listenGateway } from './proxy/server.js'
export { InstanceSupervisor } from './instance/supervisor.js'
export * from './auth/index.js'

/** @deprecated Phase 0 placeholder — removed once gateway listens. */
export function gatewayPlaceholder(): string {
  return 'avcd-agent-gateway-ready'
}

async function main(): Promise<void> {
  const settings = loadAuthSettings()
  logAuthSettingsOnStartup(settings)

  const dataRoot = process.env.AVCD_AGENT_DATA_ROOT?.trim()
  if (!dataRoot) {
    throw new Error('AVCD_AGENT_DATA_ROOT is required')
  }
  const gooseBin = process.env.GOOSE_BIN?.trim() || 'goose'
  const port = Number(process.env.PORT || process.env.GATEWAY_PORT || 3100)

  const supervisor = new InstanceSupervisor({
    gooseBin,
    instanceConfig: {
      dataRoot,
      gooseProvider: process.env.GOOSE_PROVIDER,
      gooseModel: process.env.GOOSE_MODEL,
      providerApiKeyEnv: process.env.PROVIDER_API_KEY_ENV || 'OPENROUTER_API_KEY',
      providerApiKey: process.env.OPENROUTER_API_KEY,
    },
    logger: (message) => console.log(`[gateway] ${message}`),
  })

  const { baseUrl } = await listenGateway({
    supervisor,
    settings,
    host: process.env.GATEWAY_HOST || '0.0.0.0',
    port,
  })
  console.log(`[gateway] listening on ${baseUrl}`)

  const shutdown = async (signal: string) => {
    console.log(`[gateway] ${signal} — shutting down`)
    await supervisor.stopAll()
    process.exit(0)
  }
  process.on('SIGTERM', () => void shutdown('SIGTERM'))
  process.on('SIGINT', () => void shutdown('SIGINT'))
}

const isDirectRun =
  process.argv[1] &&
  (import.meta.url === `file://${process.argv[1]}` ||
    import.meta.url.endsWith(process.argv[1].replace(/\\/g, '/')))

if (isDirectRun || process.env.AVCD_GATEWAY_MAIN === '1') {
  main().catch((err) => {
    console.error(err)
    process.exit(1)
  })
}
