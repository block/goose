import http from 'node:http'
import { AddressInfo } from 'node:net'

import { AUTH_LOGGED_OUT_PATH, AUTH_REDIRECT_PATH } from './config'

export type LoopbackResult =
  | { type: 'code'; code: string; state: string }
  | { type: 'error'; error: string; description?: string }
  | { type: 'logged_out' }

export type LoopbackServer = {
  port: number
  waitForCallback: (timeoutMs?: number) => Promise<LoopbackResult>
  close: () => Promise<void>
}

function htmlPage(title: string, body: string): string {
  return `<!doctype html><html><head><meta charset="utf-8"><title>${title}</title>
<style>body{font-family:system-ui,sans-serif;max-width:32rem;margin:4rem auto;padding:0 1rem;color:#1a1a1a}
h1{font-weight:500;font-size:1.5rem}p{color:#555}</style></head>
<body><h1>${title}</h1><p>${body}</p></body></html>`
}

export async function startLoopbackServer(port: number): Promise<LoopbackServer> {
  let resolveResult: ((result: LoopbackResult) => void) | null = null
  let rejectResult: ((err: Error) => void) | null = null
  let settled = false

  const server = http.createServer((req, res) => {
    try {
      const url = new URL(req.url ?? '/', `http://127.0.0.1:${port}`)
      if (url.pathname === AUTH_LOGGED_OUT_PATH) {
        res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' })
        res.end(
          htmlPage(
            'Signed out',
            'You can close this tab and return to Avocado Work.'
          )
        )
        if (!settled) {
          settled = true
          resolveResult?.({ type: 'logged_out' })
        }
        return
      }

      if (url.pathname !== AUTH_REDIRECT_PATH) {
        res.writeHead(404, { 'content-type': 'text/plain' })
        res.end('Not found')
        return
      }

      const error = url.searchParams.get('error')
      if (error) {
        const description = url.searchParams.get('error_description') ?? undefined
        res.writeHead(400, { 'content-type': 'text/html; charset=utf-8' })
        res.end(
          htmlPage(
            'Sign-in failed',
            description || error || 'Authorization was denied.'
          )
        )
        if (!settled) {
          settled = true
          resolveResult?.({ type: 'error', error, description })
        }
        return
      }

      const code = url.searchParams.get('code')
      const state = url.searchParams.get('state')
      if (!code || !state) {
        res.writeHead(400, { 'content-type': 'text/plain' })
        res.end('Missing code or state')
        return
      }

      res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' })
      res.end(
        htmlPage(
          'Signed in',
          'You can close this tab and return to Avocado Work.'
        )
      )
      if (!settled) {
        settled = true
        resolveResult?.({ type: 'code', code, state })
      }
    } catch (error) {
      res.writeHead(500, { 'content-type': 'text/plain' })
      res.end('Internal error')
      if (!settled) {
        settled = true
        rejectResult?.(error instanceof Error ? error : new Error(String(error)))
      }
    }
  })

  await new Promise<void>((resolve, reject) => {
    server.once('error', reject)
    server.listen(port, '127.0.0.1', () => resolve())
  })

  const addr = server.address() as AddressInfo

  return {
    port: addr.port,
    waitForCallback(timeoutMs = 5 * 60_000) {
      return new Promise<LoopbackResult>((resolve, reject) => {
        if (settled) {
          reject(new Error('Callback already consumed'))
          return
        }
        resolveResult = resolve
        rejectResult = reject
        const timer = setTimeout(() => {
          if (!settled) {
            settled = true
            reject(new Error('Timed out waiting for OAuth callback'))
          }
        }, timeoutMs)
        const clear = () => clearTimeout(timer)
        const origResolve = resolve
        const origReject = reject
        resolveResult = (r) => {
          clear()
          origResolve(r)
        }
        rejectResult = (e) => {
          clear()
          origReject(e)
        }
      })
    },
    close() {
      return new Promise((resolve, reject) => {
        server.close((err) => (err ? reject(err) : resolve()))
      })
    },
  }
}
