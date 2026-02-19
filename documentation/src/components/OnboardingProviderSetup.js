import React from "react";

export const OnboardingProviderSetup = () => {
  return (
    <>
      <ul>
        <li><strong>Quick Setup with API Key</strong> - goose will automatically configure your provider based on your API key</li>
        <li><strong><a href="https://chatgpt.com/codex">ChatGPT Subscription</a></strong> - Sign in with your ChatGPT Plus/Pro credentials to access GPT-5 Codex models</li>
        <li><strong><a href="https://tetrate.io/products/tetrate-agent-router-service">Agent Router by Tetrate</a></strong> - Access multiple AI models with automatic setup</li>
        <li><strong><a href="https://openrouter.ai/">OpenRouter</a></strong> - Access 200+ models with one API using pay-per-use pricing</li>
        <li><strong>Other Providers</strong> - Manually configure additional providers through settings</li>
      </ul>
    </>
  );
};
