import React from "react";

export const OnboardingProviderSetup = () => {
  return (
    <>
      <ul>
        <li>
          <strong>Tetrate Agent Router</strong> - One-click OAuth authentication provides instant access to multiple AI models, starting credits, and built-in rate limiting. See the <a href="/goose/docs/quickstart#configure-provider"> goose quickstart guide</a> for a walkthrough of this setup.
          <div className="admonition admonition-info alert alert--info" style={{marginTop: '0.5rem', marginBottom: '0.5rem'}}>
            <div className="admonition-heading">
              <h5>
                <span className="admonition-icon" style={{marginRight: '0.5rem'}}>
                  <svg viewBox="0 0 14 16" width="21" height="23" style={{transform: 'translateY(1px)'}}>
                    <path fillRule="evenodd" d="M7 2.3c3.14 0 5.7 2.56 5.7 5.7s-2.56 5.7-5.7 5.7A5.71 5.71 0 0 1 1.3 8c0-3.14 2.56-5.7 5.7-5.7zM7 1C3.14 1 0 4.14 0 8s3.14 7 7 7 7-3.14 7-7-3.14-7-7-7zm1 3H6v5h2V4zm0 6H6v2h2v-2z"></path>
                  </svg>
                </span>
                INFO
              </h5>
            </div>
            <div className="admonition-content" style={{paddingBottom: '1rem'}}>
              <p style={{marginBottom: '0'}}>You'll receive $10 in free credits the first time you automatically authenticate with Tetrate through Goose. This offer is available to both new and existing Tetrate users.</p>
            </div>
          </div>
        </li>
        <li><strong>OpenRouter</strong> - One-click OAuth authentication provides instant access to multiple AI models with built-in rate limiting.</li>
        <li><strong>Other Providers</strong> - Choose from a selection of <a href="/goose/docs/getting-started/providers">~20 supported providers</a> including OpenAI, Anthropic, Google Gemini, and others through manual configuration. If you don't see your provider in the list, you can add a custom provider. Be ready to provide your API key, API Host address, or other optional parameters depending on provider.</li>
      </ul>
    </>
  );
};
