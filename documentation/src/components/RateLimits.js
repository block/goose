import React from "react";
import Link from "@docusaurus/Link";
import Admonition from "@theme/Admonition";

export const RateLimits = ({
  title = "Billing",
  providerLinkText = "Google Gemini",
  providerLinkHref = "https://aistudio.google.com/app/apikey",
  providerText =
    " offers a free tier you can get started with. Otherwise, you'll need to ensure that you have credits available in your LLM Provider account to successfully make requests.",
  rateLimitsText =
    "Some providers also have rate limits on API usage, which can affect your experience. Check out our ",
  guideLinkText = "Handling Rate Limits",
  guideHref = "/docs/guides/handling-llm-rate-limits-with-goose",
  guideText =
    " guide to learn how to efficiently manage these limits while using goose.",
}) => {
  const isExternalProviderLink = /^(?:[a-z]+:)?\/\//i.test(providerLinkHref);

  return (
    <Admonition type="info" title={title}>
      {isExternalProviderLink ? (
        <a
          href={providerLinkHref}
          target="_blank"
          rel="noopener noreferrer"
        >
          {providerLinkText}
        </a>
      ) : (
        <Link to={providerLinkHref}>{providerLinkText}</Link>
      )}{" "}
      {providerText}
      <br />
      <br />
      {rateLimitsText}
      <Link to={guideHref}>{guideLinkText}</Link>
      {guideText}
    </Admonition>
  );
};
