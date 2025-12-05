#!/usr/bin/env node

/**
 * Simple static file server for testing markdown exports locally.
 * Unlike `docusaurus serve`, this serves files as-is without routing logic.
 */

const http = require('http');
const serveStatic = require('serve-static');
const path = require('path');

const buildDir = path.join(__dirname, '..', 'build');
const port = process.env.PORT || 3001;

const serve = serveStatic(buildDir, {
  index: ['index.html'],
  setHeaders: (res, filePath) => {
    // Set proper content type for markdown files
    if (filePath.endsWith('.md')) {
      res.setHeader('Content-Type', 'text/plain; charset=utf-8');
    }
  }
});

const server = http.createServer((req, res) => {
  // Handle requests to /goose/ by serving from the build directory
  if (req.url.startsWith('/goose/')) {
    // Strip /goose/ prefix and serve the file
    req.url = req.url.substring(6); // Remove '/goose'
    serve(req, res, () => {
      res.statusCode = 404;
      res.end('Not found');
    });
  } else if (req.url === '/') {
    // Redirect root to /goose/
    res.writeHead(302, { Location: '/goose/' });
    res.end();
  } else {
    // For any other path, return 404
    res.statusCode = 404;
    res.end('Not found - try /goose/');
  }
});

server.listen(port, () => {
  console.log(`\n🚀 Static file server running at http://localhost:${port}`);
  console.log(`\n🏠 Homepage: http://localhost:${port}/goose/`);
  console.log(`\n📝 Test markdown exports:`);
  console.log(`   http://localhost:${port}/goose/docs/quickstart.md`);
  console.log(`   http://localhost:${port}/goose/docs/getting-started/installation.md\n`);
});
