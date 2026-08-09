import http from 'node:http'
import net from 'node:net'
import { URL } from 'node:url'

import type { InstanceSupervisor } from '../instance/supervisor.js'
import {
  BearerAuthError,
  ForbiddenError,
  checkJwksReachability,
  extractAccessToken,
  getAuthMetrics,
  loadAuthSettings,
  recordAuthFailure,
  resolveInstanceKey,
  type AuthSettings,
  verifyBearerToken,
} from '../auth/index.js'

export type GatewayServerOptions = {
  supervisor: InstanceSupervisor
  settings?: AuthSettings
  host?: string
  port?: number
}

const HOP_BY_HOP = new Set([
  'connection',
  'keep-alive',
  'proxy-authenticate',
  'proxy-authorization',
  'te',
  'trailers',
  'transfer-encoding',
  'upgrade',
  'host',
  'origin',
  'authorization',
])

function getQueryToken(url: URL): string | undefined {
  return url.searchParams.get('token') ?? undefined
}

function rewriteUpstreamUrl(reqUrl: string, instanceBase: string, secret: string): URL {
  const incoming = new URL(reqUrl, 'http://gateway.local')
  const upstream = new URL(instanceBase)
  upstream.pathname = incoming.pathname
  upstream.search = ''
  for (const [k, v] of incoming.searchParams) {
    if (k === 'token') continue
    upstream.searchParams.append(k, v)
  }
  // Internal auth: query token for WS clients; HTTP uses X-Secret-Key header.
  if (incoming.pathname === '/acp' || incoming.pathname.startsWith('/acp')) {
    // keep secret out of non-WS unless needed; WS upgrade uses query
  }
  void secret
  return upstream
}

export function createGatewayServer(opts: GatewayServerOptions): http.Server {
  const settings = opts.settings ?? loadAuthSettings()
  const supervisor = opts.supervisor

  const server = http.createServer(async (req, res) => {
    try {
      const host = req.headers.host ?? 'localhost'
      const url = new URL(req.url ?? '/', `http://${host}`)

      if (req.method === 'OPTIONS') {
        res.writeHead(204, {
          'access-control-allow-origin': req.headers.origin ?? '*',
          'access-control-allow-methods': 'GET,POST,DELETE,OPTIONS',
          'access-control-allow-headers':
            'authorization,content-type,accept,x-secret-key,acp-connection-id,acp-session-id',
          'access-control-expose-headers': 'acp-connection-id,acp-session-id',
        })
        res.end()
        return
      }

      if (url.pathname === '/healthz' || url.pathname === '/status') {
        res.writeHead(200, { 'content-type': 'application/json' })
        res.end(JSON.stringify({ status: 'ok', auth: getAuthMetrics() }))
        return
      }

      if (url.pathname === '/readyz') {
        const jwks = settings.jwtRequired
          ? await checkJwksReachability(settings.zitadelIssuer)
          : { ok: true as const, jwksUrl: '' }
        const code = jwks.ok ? 200 : 503
        res.writeHead(code, { 'content-type': 'application/json' })
        res.end(JSON.stringify({ status: jwks.ok ? 'ready' : 'not_ready', jwks }))
        return
      }

      if (url.pathname === '/auth/logout' && req.method === 'POST') {
        const token = extractAccessToken(
          typeof req.headers.authorization === 'string'
            ? req.headers.authorization
            : null,
          getQueryToken(url)
        )
        const payload = await verifyBearerToken(token, settings)
        const instanceKey = resolveInstanceKey(payload, settings)
        await supervisor.stop(instanceKey.key)
        res.writeHead(200, { 'content-type': 'application/json' })
        res.end(JSON.stringify({ ok: true, stopped: instanceKey.key }))
        return
      }

      const isProxied =
        url.pathname === '/acp' ||
        url.pathname.startsWith('/acp') ||
        url.pathname.startsWith('/mcp-app')

      if (!isProxied) {
        res.writeHead(404, { 'content-type': 'text/plain' })
        res.end('not found')
        return
      }

      const token = extractAccessToken(
        typeof req.headers.authorization === 'string'
          ? req.headers.authorization
          : null,
        getQueryToken(url)
      )
      const payload = await verifyBearerToken(token, settings)
      const instanceKey = resolveInstanceKey(payload, settings)
      const instance = await supervisor.getOrStart(instanceKey)

      const upstreamUrl = rewriteUpstreamUrl(req.url ?? '/', instance.baseUrl, instance.secretKey)
      // For non-WS HTTP, authenticate with header; do not put secret in query.
      const headers: Record<string, string> = {
        'x-secret-key': instance.secretKey,
      }
      for (const [name, value] of Object.entries(req.headers)) {
        if (!value) continue
        const lower = name.toLowerCase()
        if (HOP_BY_HOP.has(lower)) continue
        if (lower === 'x-secret-key') continue
        headers[name] = Array.isArray(value) ? value.join(',') : value
      }
      // Critical: Accept must be forwarded verbatim for SSE.
      if (req.headers.accept) {
        headers.accept = Array.isArray(req.headers.accept)
          ? req.headers.accept.join(',')
          : req.headers.accept
      }
      if (req.headers['x-forwarded-proto']) {
        headers['x-forwarded-proto'] = String(req.headers['x-forwarded-proto'])
      } else if (
        'encrypted' in req.socket &&
        Boolean((req.socket as net.Socket & { encrypted?: boolean }).encrypted)
      ) {
        headers['x-forwarded-proto'] = 'https'
      } else {
        headers['x-forwarded-proto'] = 'http'
      }
      // Never forward Origin to goose.
      delete headers.origin

      const chunks: Buffer[] = []
      for await (const chunk of req) {
        chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk))
      }
      const body = Buffer.concat(chunks)

      const upstream = await fetch(upstreamUrl, {
        method: req.method,
        headers,
        body: req.method === 'GET' || req.method === 'HEAD' ? undefined : body,
        redirect: 'manual',
      })

      const outHeaders: Record<string, string> = {}
      upstream.headers.forEach((value, key) => {
        if (key.toLowerCase() === 'transfer-encoding') return
        outHeaders[key] = value
      })
      // Disable buffering hints for SSE
      if ((outHeaders['content-type'] ?? '').includes('text/event-stream')) {
        outHeaders['x-accel-buffering'] = 'no'
        outHeaders['cache-control'] = 'no-cache'
      }

      res.writeHead(upstream.status, outHeaders)
      if (!upstream.body) {
        res.end()
        return
      }
      const reader = upstream.body.getReader()
      while (true) {
        const { done, value } = await reader.read()
        if (done) break
        res.write(Buffer.from(value))
      }
      res.end()
    } catch (error) {
      if (error instanceof ForbiddenError) {
        recordAuthFailure()
        res.writeHead(403, { 'content-type': 'application/json' })
        res.end(JSON.stringify({ error: error.message }))
        return
      }
      if (error instanceof BearerAuthError) {
        recordAuthFailure()
        res.writeHead(error.statusCode, {
          'content-type': 'application/json',
          'www-authenticate': error.wwwAuthenticate,
        })
        res.end(JSON.stringify({ error: error.message }))
        return
      }
      console.error('[gateway] request failed', error)
      res.writeHead(502, { 'content-type': 'application/json' })
      res.end(JSON.stringify({ error: 'Bad gateway' }))
    }
  })

  server.on('upgrade', async (req, socket, head) => {
    try {
      const host = req.headers.host ?? 'localhost'
      const url = new URL(req.url ?? '/', `http://${host}`)
      if (url.pathname !== '/acp' && !url.pathname.startsWith('/acp')) {
        socket.write('HTTP/1.1 404 Not Found\r\n\r\n')
        socket.destroy()
        return
      }

      const token = extractAccessToken(
        typeof req.headers.authorization === 'string'
          ? req.headers.authorization
          : null,
        getQueryToken(url)
      )
      const payload = await verifyBearerToken(token, settings)
      const instanceKey = resolveInstanceKey(payload, settings)
      const instance = await supervisor.getOrStart(instanceKey)

      const upstreamUrl = new URL(instance.baseUrl)
      upstreamUrl.pathname = url.pathname
      upstreamUrl.search = ''
      for (const [k, v] of url.searchParams) {
        if (k === 'token') continue
        upstreamUrl.searchParams.append(k, v)
      }
      upstreamUrl.searchParams.set('token', instance.secretKey)

      const proxySocket = net.connect(instance.port, '127.0.0.1', () => {
        const lines = [
          `GET ${upstreamUrl.pathname}${upstreamUrl.search} HTTP/1.1`,
          `Host: 127.0.0.1:${instance.port}`,
          'Connection: Upgrade',
          'Upgrade: websocket',
        ]
        for (const [name, value] of Object.entries(req.headers)) {
          if (!value) continue
          const lower = name.toLowerCase()
          if (lower === 'host' || lower === 'origin' || lower === 'authorization') continue
          if (lower === 'connection' || lower === 'upgrade') continue
          const v = Array.isArray(value) ? value.join(',') : value
          lines.push(`${name}: ${v}`)
        }
        // Do not send Origin — goose accepts missing Origin in all modes.
        lines.push('', '')
        proxySocket.write(lines.join('\r\n'))
        if (head.length) proxySocket.write(head)
        socket.pipe(proxySocket)
        proxySocket.pipe(socket)
      })

      proxySocket.on('error', () => {
        socket.destroy()
      })
      socket.on('error', () => {
        proxySocket.destroy()
      })
    } catch (error) {
      recordAuthFailure()
      const status =
        error instanceof ForbiddenError
          ? 403
          : error instanceof BearerAuthError
            ? error.statusCode
            : 401
      const www =
        error instanceof BearerAuthError
          ? error.wwwAuthenticate
          : 'Bearer error="invalid_token"'
      socket.write(
        `HTTP/1.1 ${status} Unauthorized\r\nWWW-Authenticate: ${www}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n`
      )
      socket.destroy()
    }
  })

  return server
}

export async function listenGateway(
  opts: GatewayServerOptions
): Promise<{ server: http.Server; port: number; baseUrl: string }> {
  const server = createGatewayServer(opts)
  const host = opts.host ?? '127.0.0.1'
  const port = opts.port ?? 0
  await new Promise<void>((resolve, reject) => {
    server.listen(port, host, () => resolve())
    server.once('error', reject)
  })
  const addr = server.address()
  if (!addr || typeof addr === 'string') {
    throw new Error('Failed to bind gateway')
  }
  return {
    server,
    port: addr.port,
    baseUrl: `http://${host}:${addr.port}`,
  }
}
