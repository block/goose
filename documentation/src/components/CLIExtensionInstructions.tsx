import React from 'react';
import CodeBlock from '@theme/CodeBlock';
import Admonition from '@theme/Admonition';

interface EnvVar {
  key: string;
  value: string;
}

interface CLIExtensionInstructionsProps {
  name: string;
  description: string;
  type?: 'stdio' | 'sse' | 'http';
  command?: string; // Only for stdio
  url?: string; // For both sse and http
  timeout?: number;
  envVars?: EnvVar[]; // For stdio: environment variables, for http: headers
  infoNote?: string;
  commandNote?: React.ReactNode; // Note to display for command/URL step
}

export default function CLIExtensionInstructions({
  name,
  description,
  type = 'stdio',
  command,
  url,
  timeout = 300,
  envVars = [],
  infoNote,
  commandNote,
}: CLIExtensionInstructionsProps) {
  const hasEnvVars = envVars.length > 0;
  const isSSE = type === 'sse';
  const isHttp = type === 'http';
  const isRemote = isSSE || isHttp;

  // Determine last-step prompt text
  const lastStepText = isHttp
    ? 'Would you like to add custom headers?'
    : 'Would you like to add environment variables?';

  const lastStepInstruction = hasEnvVars
    ? `Add ${isHttp ? 'custom header' : 'environment variable'}${envVars.length > 1 ? 's' : ''} for ${name}`
    : isHttp
    ? 'Choose No when asked to add custom headers.'
    : 'Choose No when asked to add environment variables.';

  return (
    <div>
      <ol>
        <li>Run the <code>configure</code> command:</li>
      </ol>
      <CodeBlock language="sh">{`goose configure`}</CodeBlock>

      <ol start={2}>
        <li>
          Choose to add a{' '}
          <code>
            {isSSE 
              ? 'Remote Extension (SSE)' 
              : isHttp 
              ? 'Remote Extension (Streaming HTTP)' 
              : 'Command-line Extension'
            }
          </code>.
        </li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension 
│
◆  What type of extension would you like to add?
${
  isSSE
    ? '│  ○ Built-in Extension\n│  ○ Command-line Extension\n// highlight-start\n│  ● Remote Extension (SSE) (Connect to a remote extension via Server-Sent Events)\n// highlight-end\n│  ○ Remote Extension (Streaming HTTP)'
    : isHttp
    ? '│  ○ Built-in Extension\n│  ○ Command-line Extension\n│  ○ Remote Extension (SSE)\n// highlight-start\n│  ● Remote Extension (Streaming HTTP) (Connect to a remote extension via MCP Streaming HTTP)\n// highlight-end'
    : '│  ○ Built-in Extension\n// highlight-start\n│  ● Command-line Extension (Run a local command or script)\n// highlight-end\n│  ○ Remote Extension (SSE)\n│  ○ Remote Extension (Streaming HTTP)'
}
└`}</CodeBlock>

      <ol start={3}>
        <li>Give your extension a name.</li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension
│
◇  What type of extension would you like to add?
│  ${isSSE ? 'Remote Extension (SSE)' : isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
// highlight-start
◆  What would you like to call this extension?
│  ${name}
// highlight-end
└`}</CodeBlock>

      {isRemote ? (
        <>
          <ol start={4}>
            <li>Enter the {isSSE ? 'SSE endpoint URI' : 'Streaming HTTP endpoint URI'}.</li>
          </ol>
          {commandNote && (
            <>
              <Admonition type="info">
                {commandNote}
              </Admonition>
              <br />
            </>
          )}
          <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension 
│
◇  What type of extension would you like to add?
│  ${isSSE ? 'Remote Extension (SSE)' : 'Remote Extension (Streaming HTTP)'}
│
◇  What would you like to call this extension?
│  ${name}
│
// highlight-start
◆  What is the ${isSSE ? 'SSE endpoint URI' : 'Streaming HTTP endpoint URI'}?
│  ${url}
// highlight-end
└`}</CodeBlock>
        </>
      ) : (
        <>
          <ol start={4}>
            <li>Enter the command to run when this extension is used.</li>
          </ol>
          {commandNote && (
            <>
              <Admonition type="info">
                {commandNote}
              </Admonition>
              <br />
            </>
          )}
          <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension
│
◇  What type of extension would you like to add?
│  Command-line Extension 
│
◇  What would you like to call this extension?
│  ${name}
│
// highlight-start
◆  What command should be run?
│  ${command}
// highlight-end
└`}</CodeBlock>
        </>
      )}

      <ol start={5}>
        <li>
          Enter the number of seconds Goose should wait for actions to complete before timing out. Default is{' '}
          <code>300</code> seconds.
        </li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension
│
◇  What type of extension would you like to add?
│  ${isSSE ? 'Remote Extension (SSE)' : isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
◇  What would you like to call this extension?
│  ${name}
│
${
  isRemote
    ? `◇  What is the ${isSSE ? 'SSE endpoint URI' : 'Streaming HTTP endpoint URI'}?\n│  ${url}\n│`
    : `◇  What command should be run?\n│  ${command}\n│`
}
// highlight-start
◆  Please set the timeout for this tool (in secs):
│  ${timeout}
// highlight-end
└`}</CodeBlock>

      <ol start={6}>
        <li>Enter a description for this extension.</li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension
│
◇  What type of extension would you like to add?
│  ${isSSE ? 'Remote Extension (SSE)' : isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
◇  What would you like to call this extension?
│  ${name}
│
${
  isRemote
    ? `◇  What is the ${isSSE ? 'SSE endpoint URI' : 'Streaming HTTP endpoint URI'}?\n│  ${url}\n│`
    : `◇  What command should be run?\n│  ${command}\n│`
}
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
// highlight-start
◆  Enter a description for this extension:
│  ${description}
// highlight-end
└`}</CodeBlock>

      <ol start={7}>
        <li>
          {hasEnvVars
            ? isHttp
              ? <>Add {envVars.length > 1 ? 'custom headers' : 'a custom header'} for this extension.</>
              : <>Add {envVars.length > 1 ? 'environment variables' : 'an environment variable'} for this extension.</>
            : isHttp
            ? <>Choose <code>No</code> when asked to add custom headers.</>
            : <>Choose <code>No</code> when asked to add environment variables.</>
          }
        </li>
      </ol>

      {!hasEnvVars && (
        <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension 
│
◇  What type of extension would you like to add?
│  ${isSSE ? 'Remote Extension (SSE)' : isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
◇  What would you like to call this extension?
│  ${name}
│
${
  isRemote
    ? `◇  What is the ${isSSE ? 'SSE endpoint URI' : 'Streaming HTTP endpoint URI'}?\n│  ${url}\n│`
    : `◇  What command should be run?\n│  ${command}\n│`
}
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
◇  Enter a description for this extension:
│  ${description}
│
// highlight-start
◆  ${lastStepText}
│  No
// highlight-end
│
└  Added ${name} extension`}</CodeBlock>
      )}

      {hasEnvVars && (
        <>
          {infoNote && (
            <>
              <Admonition type="info">
                {infoNote}
              </Admonition>
              <br />
            </>
          )}

          <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension
│
◇  What type of extension would you like to add?
│  ${isSSE ? 'Remote Extension (SSE)' : isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
◇  What would you like to call this extension?
│  ${name}
│
${
  isRemote
    ? `◇  What is the ${isSSE ? 'SSE endpoint URI' : 'Streaming HTTP endpoint URI'}?\n│  ${url}\n│`
    : `◇  What command should be run?\n│  ${command}\n│`
}
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
◇  Enter a description for this extension:
│  ${description}
│
// highlight-start
◆  ${lastStepText}
│  Yes
${envVars
  .map(
    ({ key, value }, i) => `│
◇  ${isHttp ? 'Header name' : 'Environment variable name'}:
│  ${key}
│
◇  ${isHttp ? 'Header value' : 'Environment variable value'}:
│  ${value}
│
◇  Add another ${isHttp ? 'header' : 'environment variable'}?
│  ${i === envVars.length - 1 ? 'No' : 'Yes'}`
  )
  .join('\n')}
// highlight-end
│
└  Added ${name} extension`}</CodeBlock>
        </>
      )}
    </div>
  );
}
