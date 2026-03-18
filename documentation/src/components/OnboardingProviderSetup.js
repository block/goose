import React from "react";

const renderLabel = ({ label, href }) => {
  if (!href) {
    return label;
  }

  return <a href={href}>{label}</a>;
};

export const OnboardingProviderSetup = ({
  quickSetupLabel = "Quick Setup with API Key",
  quickSetupDescription = "goose will automatically configure your provider based on your API key",
  chatGptLabel = "ChatGPT Subscription",
  chatGptHref = "https://chatgpt.com/codex",
  chatGptDescription = "Sign in with your ChatGPT Plus/Pro credentials to access GPT-5 Codex models",
  agentRouterLabel = "Agent Router by Tetrate",
  agentRouterHref = "https://tetrate.io/products/tetrate-agent-router-service",
  agentRouterDescription = "Access multiple AI models with automatic setup",
  openRouterLabel = "OpenRouter",
  openRouterHref = "https://openrouter.ai/",
  openRouterDescription = "Access 200+ models with one API using pay-per-use pricing",
  otherProvidersLabel = "Other Providers",
  otherProvidersDescription = "Manually configure additional providers through settings",
}) => {
  const items = [
    {
      label: quickSetupLabel,
      description: quickSetupDescription,
    },
    {
      label: chatGptLabel,
      href: chatGptHref,
      description: chatGptDescription,
    },
    {
      label: agentRouterLabel,
      href: agentRouterHref,
      description: agentRouterDescription,
    },
    {
      label: openRouterLabel,
      href: openRouterHref,
      description: openRouterDescription,
    },
    {
      label: otherProvidersLabel,
      description: otherProvidersDescription,
    },
  ];

  return (
    <ul>
      {items.map((item) => (
        <li key={item.label}>
          <strong>{renderLabel(item)}</strong> - {item.description}
        </li>
      ))}
    </ul>
  );
};
