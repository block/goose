#!/usr/bin/env node
/**
 * Local E2E Test Server
 *
 * Simulates the multi-tenant SaaS backend locally for testing subdomain routing,
 * API endpoints, and webhook handling.
 *
 * Setup:
 * 1. Add to /etc/hosts:
 *    127.0.0.1  test-app.local
 *    127.0.0.1  tenant1.test-app.local
 *    127.0.0.1  admin.test-app.local
 *
 * 2. Run: npm run test:e2e:server
 *
 * 3. Open in browser:
 *    - http://test-app.local:3000 → Landing page
 *    - http://tenant1.test-app.local:3000 → Tenant app
 *    - http://admin.test-app.local:3000 → Admin dashboard
 *
 * 4. Test webhooks:
 *    curl -X POST http://test-app.local:3000/webhooks/clerk \
 *      -H "Content-Type: application/json" \
 *      -d '{"type": "user.created", "data": {"id": "test_user"}}'
 */

import { createServer } from 'http';
import { readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PORT = parseInt(process.env.PORT || '3000', 10);
const HOST = '0.0.0.0';

// ============== In-Memory KV Storage ==============

class LocalKV {
  constructor() {
    this.store = new Map();
  }

  async get(key) {
    return this.store.get(key) || null;
  }

  async put(key, value) {
    this.store.set(key, value);
  }

  async delete(key) {
    this.store.delete(key);
  }

  async list({ prefix = '' } = {}) {
    const keys = Array.from(this.store.keys())
      .filter(k => k.startsWith(prefix))
      .map(name => ({ name }));
    return { keys, list_complete: true };
  }

  dump() {
    return Object.fromEntries(this.store);
  }

  clear() {
    this.store.clear();
  }
}

// ============== Environment ==============

const TENANTS = new LocalKV();

const env = {
  TENANTS,
  APP_DOMAIN: 'test-app.local',
  PAGES_HOSTNAME: 'localhost',
  MONTHLY_PRICE: '9',
  YEARLY_PRICE: '89'
};

function prefixKey(key) {
  return `${env.APP_DOMAIN}:${key}`;
}

// ============== Request Helpers ==============

function parseBody(req) {
  return new Promise((resolve, reject) => {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      try {
        resolve(body ? JSON.parse(body) : {});
      } catch (e) {
        reject(e);
      }
    });
    req.on('error', reject);
  });
}

function sendJson(res, data, status = 200) {
  res.writeHead(status, {
    'Content-Type': 'application/json',
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type'
  });
  res.end(JSON.stringify(data, null, 2));
}

function sendHtml(res, html, status = 200) {
  res.writeHead(status, { 'Content-Type': 'text/html' });
  res.end(html);
}

// ============== Route Handlers ==============

async function handleApiTenants(req, res) {
  const listResult = await TENANTS.get(prefixKey('tenants:list'));
  const subdomains = listResult ? JSON.parse(listResult) : [];

  const tenants = [];
  for (const subdomain of subdomains) {
    const tenant = await TENANTS.get(prefixKey(`tenant:${subdomain}`));
    if (tenant) {
      tenants.push(JSON.parse(tenant));
    }
  }

  sendJson(res, { tenants });
}

async function handleApiStats(req, res) {
  const [tenantCount, userCount, subscriberCount, mrr] = await Promise.all([
    TENANTS.get(prefixKey('stats:tenantCount')),
    TENANTS.get(prefixKey('stats:userCount')),
    TENANTS.get(prefixKey('stats:subscriberCount')),
    TENANTS.get(prefixKey('stats:mrr'))
  ]);

  sendJson(res, {
    tenantCount: parseInt(tenantCount || '0'),
    userCount: parseInt(userCount || '0'),
    subscriberCount: parseInt(subscriberCount || '0'),
    mrr: parseFloat(mrr || '0')
  });
}

async function handleApiRegister(req, res) {
  const body = await parseBody(req);
  const { subdomain, userId, email, plan } = body;

  if (!subdomain || !userId) {
    return sendJson(res, { error: 'Missing subdomain or userId' }, 400);
  }

  // Check if taken by different user
  const existingStr = await TENANTS.get(prefixKey(`tenant:${subdomain}`));
  if (existingStr) {
    const existing = JSON.parse(existingStr);
    if (existing.userId !== userId) {
      return sendJson(res, { error: 'Subdomain already taken' }, 409);
    }
    existing.lastVisit = new Date().toISOString();
    await TENANTS.put(prefixKey(`tenant:${subdomain}`), JSON.stringify(existing));
    return sendJson(res, { success: true, tenant: existing, updated: true });
  }

  // Create new tenant
  const tenant = {
    subdomain,
    userId,
    email,
    plan: plan || 'pro',
    status: 'active',
    createdAt: new Date().toISOString(),
    lastVisit: new Date().toISOString()
  };

  await TENANTS.put(prefixKey(`tenant:${subdomain}`), JSON.stringify(tenant));

  // Update list
  const listResult = await TENANTS.get(prefixKey('tenants:list'));
  const subdomains = listResult ? JSON.parse(listResult) : [];
  if (!subdomains.includes(subdomain)) {
    subdomains.push(subdomain);
    await TENANTS.put(prefixKey('tenants:list'), JSON.stringify(subdomains));
    await TENANTS.put(prefixKey('stats:tenantCount'), String(subdomains.length));
  }

  sendJson(res, { success: true, tenant, created: true });
}

async function handleWebhook(req, res) {
  const body = await parseBody(req);
  const { type, data } = body;

  console.log(`[Webhook] ${type}:`, JSON.stringify(data).slice(0, 100));

  if (type === 'user.created') {
    await TENANTS.put(prefixKey(`user:${data.id}`), JSON.stringify({
      id: data.id,
      email: data.email_addresses?.[0]?.email_address,
      createdAt: new Date().toISOString()
    }));

    const count = parseInt(await TENANTS.get(prefixKey('stats:userCount')) || '0');
    await TENANTS.put(prefixKey('stats:userCount'), String(count + 1));
  }

  if (type === 'user.deleted') {
    await TENANTS.delete(prefixKey(`user:${data.id}`));
    const count = parseInt(await TENANTS.get(prefixKey('stats:userCount')) || '0');
    await TENANTS.put(prefixKey('stats:userCount'), String(Math.max(0, count - 1)));
  }

  sendJson(res, { received: true });
}

// ============== Debug Endpoints ==============

async function handleDebugKv(req, res) {
  const data = TENANTS.dump();
  sendJson(res, data);
}

async function handleDebugReset(req, res) {
  TENANTS.clear();
  sendJson(res, { reset: true });
}

// ============== Static Pages ==============

function getLandingPage() {
  return `<!DOCTYPE html>
<html>
<head>
  <title>Test App - Landing</title>
  <style>
    body { font-family: system-ui; max-width: 800px; margin: 40px auto; padding: 20px; }
    h1 { color: #333; }
    .subdomain { background: #f0f0f0; padding: 10px; border-radius: 4px; margin: 10px 0; }
    code { background: #e0e0e0; padding: 2px 6px; border-radius: 3px; }
  </style>
</head>
<body>
  <h1>Welcome to Test App</h1>
  <p>This is the landing page at the root domain.</p>

  <h2>Test Subdomains</h2>
  <div class="subdomain">
    <strong>Tenant:</strong> <a href="http://tenant1.test-app.local:${PORT}">tenant1.test-app.local:${PORT}</a>
  </div>
  <div class="subdomain">
    <strong>Admin:</strong> <a href="http://admin.test-app.local:${PORT}">admin.test-app.local:${PORT}</a>
  </div>

  <h2>API Endpoints</h2>
  <ul>
    <li><code>GET /api/tenants</code> - List all tenants</li>
    <li><code>GET /api/stats</code> - Get stats</li>
    <li><code>POST /api/tenants/register</code> - Register tenant</li>
    <li><code>POST /webhooks/clerk</code> - Webhook endpoint</li>
  </ul>

  <h2>Debug Endpoints</h2>
  <ul>
    <li><a href="/debug/kv"><code>GET /debug/kv</code></a> - Dump KV contents</li>
    <li><code>POST /debug/reset</code> - Clear KV</li>
  </ul>
</body>
</html>`;
}

function getTenantPage(subdomain) {
  return `<!DOCTYPE html>
<html>
<head>
  <title>Tenant: ${subdomain}</title>
  <style>
    body { font-family: system-ui; max-width: 800px; margin: 40px auto; padding: 20px; }
    h1 { color: #4a90d9; }
    .info { background: #e8f4fc; padding: 15px; border-radius: 4px; }
  </style>
</head>
<body>
  <h1>Tenant Dashboard: ${subdomain}</h1>
  <div class="info">
    <p>Subdomain: <strong>${subdomain}</strong></p>
    <p>This is a tenant-specific page.</p>
  </div>

  <p><a href="http://test-app.local:${PORT}">Back to Landing</a></p>
</body>
</html>`;
}

function getAdminPage() {
  return `<!DOCTYPE html>
<html>
<head>
  <title>Admin Dashboard</title>
  <style>
    body { font-family: system-ui; max-width: 1000px; margin: 40px auto; padding: 20px; }
    h1 { color: #d94a4a; }
    .stats { display: flex; gap: 20px; margin: 20px 0; }
    .stat { background: #f8f8f8; padding: 20px; border-radius: 8px; text-align: center; }
    .stat-value { font-size: 2em; font-weight: bold; color: #333; }
    .stat-label { color: #666; }
    table { width: 100%; border-collapse: collapse; margin-top: 20px; }
    th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
    th { background: #f0f0f0; }
  </style>
</head>
<body>
  <h1>Admin Dashboard</h1>

  <div class="stats" id="stats">Loading...</div>

  <h2>Tenants</h2>
  <table id="tenants">
    <thead>
      <tr><th>Subdomain</th><th>Email</th><th>Plan</th><th>Status</th><th>Created</th></tr>
    </thead>
    <tbody></tbody>
  </table>

  <p style="margin-top: 40px;"><a href="http://test-app.local:${PORT}">Back to Landing</a></p>

  <script>
    async function loadData() {
      const [statsRes, tenantsRes] = await Promise.all([
        fetch('/api/stats'),
        fetch('/api/tenants')
      ]);

      const stats = await statsRes.json();
      const { tenants } = await tenantsRes.json();

      document.getElementById('stats').innerHTML = \`
        <div class="stat"><div class="stat-value">\${stats.tenantCount}</div><div class="stat-label">Tenants</div></div>
        <div class="stat"><div class="stat-value">\${stats.userCount}</div><div class="stat-label">Users</div></div>
        <div class="stat"><div class="stat-value">\${stats.subscriberCount}</div><div class="stat-label">Subscribers</div></div>
        <div class="stat"><div class="stat-value">$\${stats.mrr.toFixed(2)}</div><div class="stat-label">MRR</div></div>
      \`;

      const tbody = document.querySelector('#tenants tbody');
      tbody.innerHTML = tenants.length === 0 ? '<tr><td colspan="5">No tenants yet</td></tr>' :
        tenants.map(t => \`
          <tr>
            <td><a href="http://\${t.subdomain}.test-app.local:${PORT}">\${t.subdomain}</a></td>
            <td>\${t.email || '-'}</td>
            <td>\${t.plan || '-'}</td>
            <td>\${t.subscriptionStatus || t.status || '-'}</td>
            <td>\${new Date(t.createdAt).toLocaleDateString()}</td>
          </tr>
        \`).join('');
    }

    loadData();
    setInterval(loadData, 5000);
  </script>
</body>
</html>`;
}

// ============== Request Router ==============

async function handleRequest(req, res) {
  const url = new URL(req.url, `http://${req.headers.host}`);
  const hostname = req.headers.host.split(':')[0];
  const pathname = url.pathname;

  console.log(`[${req.method}] ${hostname}${pathname}`);

  // CORS preflight
  if (req.method === 'OPTIONS') {
    res.writeHead(204, {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type'
    });
    return res.end();
  }

  try {
    // API routes
    if (pathname === '/api/tenants' && req.method === 'GET') {
      return handleApiTenants(req, res);
    }
    if (pathname === '/api/stats' && req.method === 'GET') {
      return handleApiStats(req, res);
    }
    if (pathname === '/api/tenants/register' && req.method === 'POST') {
      return handleApiRegister(req, res);
    }

    // Webhook
    if (pathname === '/webhooks/clerk' && req.method === 'POST') {
      return handleWebhook(req, res);
    }

    // Debug routes
    if (pathname === '/debug/kv') {
      return handleDebugKv(req, res);
    }
    if (pathname === '/debug/reset' && req.method === 'POST') {
      return handleDebugReset(req, res);
    }

    // Static pages based on subdomain
    const subdomain = hostname.replace('.test-app.local', '').replace('.localhost', '');

    if (subdomain === 'admin') {
      return sendHtml(res, getAdminPage());
    }

    if (subdomain && subdomain !== 'test-app' && subdomain !== 'localhost') {
      return sendHtml(res, getTenantPage(subdomain));
    }

    // Landing page
    return sendHtml(res, getLandingPage());

  } catch (err) {
    console.error('Error:', err);
    sendJson(res, { error: err.message }, 500);
  }
}

// ============== Server Startup ==============

const server = createServer(handleRequest);

server.listen(PORT, HOST, () => {
  console.log(`
╔════════════════════════════════════════════════════════════╗
║          Vibes E2E Test Server Running                     ║
╠════════════════════════════════════════════════════════════╣
║                                                            ║
║  Server: http://${HOST}:${PORT}                                 ║
║                                                            ║
║  Test URLs (add to /etc/hosts first):                      ║
║  ┌──────────────────────────────────────────────────────┐  ║
║  │ Landing:  http://test-app.local:${PORT}                │  ║
║  │ Tenant:   http://tenant1.test-app.local:${PORT}        │  ║
║  │ Admin:    http://admin.test-app.local:${PORT}          │  ║
║  └──────────────────────────────────────────────────────┘  ║
║                                                            ║
║  /etc/hosts entries:                                       ║
║    127.0.0.1  test-app.local                               ║
║    127.0.0.1  tenant1.test-app.local                       ║
║    127.0.0.1  admin.test-app.local                         ║
║                                                            ║
║  Debug:                                                    ║
║    GET  /debug/kv     - Dump KV contents                   ║
║    POST /debug/reset  - Clear KV                           ║
║                                                            ║
║  Press Ctrl+C to stop                                      ║
╚════════════════════════════════════════════════════════════╝
  `);
});

server.on('error', (err) => {
  if (err.code === 'EADDRINUSE') {
    console.error(`Port ${PORT} is already in use. Try: PORT=3001 npm run test:e2e:server`);
  } else {
    console.error('Server error:', err);
  }
  process.exit(1);
});
