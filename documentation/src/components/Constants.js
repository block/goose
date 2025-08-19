import React from "react";
import Admonition from "@theme/Admonition";

export const DesktopProviderSetup = () => {
  return (
    <>
      <p>On the welcome screen, choose how to configure a provider:</p>
      <ul>
        <li><strong>OpenRouter</strong> (recommended) - One-click OAuth authentication provides instant access to multiple AI models with built-in rate limiting.</li>
        <li><strong>Ollama</strong> - Free local AI that runs privately on your computer. If needed, the setup flow will guide you through installing Ollama and downloading the recommended model.</li>
        <li><strong>Other Providers</strong> - Choose from <a href="/goose/docs/getting-started/providers">~20 supported providers</a> including OpenAI, Anthropic, Google Gemini, and others through manual configuration. Be ready to provide your API key.</li>
      </ul>
    </>
  );
};

export const ModelSelectionTip = () => {
  return (
    <p>Goose relies heavily on tool calling capabilities and currently works best with Claude 4 models.</p>
  );
};

export const RateLimits = () => {
  return (
    <Admonition type="info" title="Billing">
      <a
        href="https://aistudio.google.com/app/apikey"
        target="_blank"
        rel="noopener noreferrer"
      >
        Google Gemini
      </a>{" "}
      offers a free tier you can get started with. Otherwise, you'll need to
      ensure that you have credits available in your LLM Provider account to
      successfully make requests.
      <br />
      <br />
      Some providers also have rate limits on API usage, which can affect your
      experience. Check out our{" "}
      <a href="/goose/docs/guides/handling-llm-rate-limits-with-goose" target="_blank">
        Handling Rate Limits
      </a>{" "}
      guide to learn how to efficiently manage these limits while using Goose.
    </Admonition>
  );
};
