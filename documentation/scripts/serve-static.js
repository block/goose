#!/usr/bin/env node

/**
 * Simple static file server for testing markdown exports locally.
 * Unlike `docusaurus serve`, this serves files as-is without routing logic.
 */

const http = require('http');
const fs = require('fs');
const serveStatic = require('serve-static');
const path = require('path');

const buildDir = path.join(__dirname, '..', 'build');
const port = process.env.PORT || 3001;

function decodePathname(url) {
  const encodedPathname = (url.substring(6) || '/').split('?')[0];

  try {
    return decodeURIComponent(encodedPathname);
  } catch {
    return null;
  }
}

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
    const strippedUrl = req.url.substring(6) || '/';
    const pathname = decodePathname(req.url);

    if (pathname === null) {
      res.statusCode = 400;
      res.end('Bad request');
      return;
    }

    let servedUrl = strippedUrl;

    // Serve directory index files directly so local preview works with or without
    // a trailing slash and does not depend on browser redirect caching.
    if (!path.extname(pathname) && !pathname.endsWith('/')) {
      const candidateDir = path.join(buildDir, pathname);
      if (fs.existsSync(candidateDir) && fs.statSync(candidateDir).isDirectory()) {
        servedUrl = `${pathname}/index.html`;
      } else {
        // Preserve the /goose base path when the static handler emits redirects.
        const originalWriteHead = res.writeHead.bind(res);
        res.writeHead = (statusCode, statusMessage, headers) => {
          let nextStatusMessage = statusMessage;
          let nextHeaders = headers;

          if (typeof nextStatusMessage === 'object' && nextHeaders === undefined) {
            nextHeaders = nextStatusMessage;
            nextStatusMessage = undefined;
          }

          const location =
            (nextHeaders && nextHeaders.Location) ||
            (nextHeaders && nextHeaders.location) ||
            res.getHeader('Location') ||
            res.getHeader('location');

          if (typeof location === 'string' && location.startsWith('/') && !location.startsWith('/goose/')) {
            const rewrittenLocation = `/goose${location}`;
            if (nextHeaders) {
              if (nextHeaders.Location) {
                nextHeaders.Location = rewrittenLocation;
              } else if (nextHeaders.location) {
                nextHeaders.location = rewrittenLocation;
              } else {
                nextHeaders.Location = rewrittenLocation;
              }
            } else {
              res.setHeader('Location', rewrittenLocation);
            }
          }

          if (nextStatusMessage === undefined) {
            return originalWriteHead(statusCode, nextHeaders);
          }
          return originalWriteHead(statusCode, nextStatusMessage, nextHeaders);
        };
      }
    }

    // Strip /goose/ prefix and serve the file
    req.url = servedUrl;
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
