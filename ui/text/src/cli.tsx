#!/usr/bin/env node
import React from "react";
import { render } from "ink";
import meow from "meow";
import App from "./app.js";

const cli = meow(
  `
  Usage
    $ goose-text

  Options
    --server, -s  Server URL (default: http://127.0.0.1:3284)
    --text, -t    Send a single prompt and exit
`,
  {
    importMeta: import.meta,
    flags: {
      server: { type: "string", shortFlag: "s", default: "http://127.0.0.1:3284" },
      text: { type: "string", shortFlag: "t" },
    },
  }
);

render(<App serverUrl={cli.flags.server} initialPrompt={cli.flags.text} />);
