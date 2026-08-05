// React entry for the roam web client.
//
// Reuses goose's reference clients directly:
//  - @aaif/goose-sdk (ui/sdk): GooseClient over the roam byte-duplex
//  - @desktop (ui/desktop/src): real desktop components (MarkdownContent,
//    ToolCallStatusIndicator, Button) + the desktop Tailwind theme
//
// Still fully stateless + CDN-hostable: no backend, all state in the tab,
// all traffic browser ⇄ relay ⇄ roam host.
import "./shim";
import "./theme.css";
import React from "react";
import { createRoot } from "react-dom/client";
import { IntlProvider } from "react-intl";
// Desktop components render strings through react-intl; give them the same
// compiled English catalog the desktop ships.
import messages from "@desktop/i18n/compiled/en.json";
import initWasm, { RoamClient } from "./wasm/goose_roaming_web.js";
import { App } from "./App";

const SECRET_STORAGE_KEY = "goose-roam-secret-hex";

async function boot() {
  await initWasm();
  // Stable per-browser roam identity so the host only accepts this tab once.
  const saved = localStorage.getItem(SECRET_STORAGE_KEY) ?? undefined;
  const roam = new RoamClient(saved);
  if (!saved) localStorage.setItem(SECRET_STORAGE_KEY, roam.secretHex());

  createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <IntlProvider locale="en" messages={messages as unknown as Record<string, string>}>
        <App roam={roam} />
      </IntlProvider>
    </React.StrictMode>,
  );
}

void boot();
