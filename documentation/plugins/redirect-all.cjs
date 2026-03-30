const fs = require('fs');
const path = require('path');

const REDIRECT_TARGET = 'https://aaif-goose.github.io';

module.exports = function redirectAllPlugin(context, options) {
  return {
    name: 'redirect-all',

    async postBuild({ outDir }) {
      const { globby } = await import('globby');
      const htmlFiles = await globby('**/*.html', { cwd: outDir });

      let count = 0;
      for (const file of htmlFiles) {
        const filePath = path.join(outDir, file);
        let html = fs.readFileSync(filePath, 'utf-8');

        // Derive the page path from the file path
        // e.g. "docs/quickstart/index.html" -> "/goose/docs/quickstart/"
        //      "index.html" -> "/goose/"
        const baseUrl = context.baseUrl || '/goose/';
        let pagePath;
        if (file === 'index.html') {
          pagePath = baseUrl;
        } else if (file.endsWith('/index.html')) {
          pagePath = baseUrl + file.replace(/\/index\.html$/, '/');
        } else {
          pagePath = baseUrl + file;
        }

        const targetUrl = REDIRECT_TARGET + pagePath;
        const metaTag = `<meta http-equiv="refresh" content="0; url=${targetUrl}">`;

        // Insert the meta tag right after <head> (or <head ...>)
        html = html.replace(/<head(\s[^>]*)?>/, `$&\n    ${metaTag}`);

        fs.writeFileSync(filePath, html);
        count++;
      }

      console.log(`[redirect-all] Injected meta redirect into ${count} HTML files -> ${REDIRECT_TARGET}`);
    },
  };
};
