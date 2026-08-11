import { loadAuthSettings, logAuthSettingsOnStartup } from './auth/index.js'
import { assertFailClosedBootEnv } from './boot-guard.js'
import { InstanceSupervisor } from './instance/supervisor.js'
import { listenGateway } from './proxy/server.js'

export { createGatewayServer, listenGateway } from './proxy/server.js'
export { InstanceSupervisor } from './instance/supervisor.js'
export { assertFailClosedBootEnv, BootConfigError } from './boot-guard.js'
export * from './auth/index.js'

/** @deprecated Phase 0 placeholder — removed once gateway listens. */
export function gatewayPlaceholder(): string {
  return 'avcd-agent-gateway-ready'
}

async function main(): Promise<void> {
  assertFailClosedBootEnv(process.env)

  const settings = loadAuthSettings()
  logAuthSettingsOnStartup(settings)

  const dataRoot = process.env.AVCD_AGENT_DATA_ROOT!.trim()
  const gooseBin = process.env.GOOSE_BIN?.trim() || 'goose'
  const port = Number(process.env.PORT || process.env.GATEWAY_PORT || 3100)
  const avocadoProvisionUrl = process.env.AVOCADO_PROVISION_URL!.trim()
  const avocadoHost =
    process.env.AVOCADO_HOST?.trim() || 'https://dev.avocado.tech/llm'

  const supervisor = new InstanceSupervisor({
    gooseBin,
    instanceConfig: {
      dataRoot,
      gooseProvider: process.env.GOOSE_PROVIDER,
      gooseModel: process.env.GOOSE_MODEL,
      // No shared OPENROUTER_API_KEY — every child is avocado-provisioned (AC-5).
    },
    avocadoProvisionUrl,
    avocadoHost,
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
