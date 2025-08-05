import React from 'react';
import CodeBlock from '@theme/CodeBlock';

interface EnvVar {
  key: string;
  value: string;
}

interface CLIExtensionInstructionsProps {
  name: string;
  type?: 'stdio' | 'http';
  command?: string; // Only for stdio
  url?: string; // Only for http
  timeout?: number;
  envVars?: EnvVar[];
  infoNote?: string;
}

export default function CLIExtensionInstructions({
  name,
  type = 'stdio',
  command,
  url,
  timeout = 300,
  envVars = [],
  infoNote,
}: CLIExtensionInstructionsProps) {
  const hasEnvVars = envVars.length > 0;
  const isHttp = type === 'http';

  // Determine last-step prompt text
  const lastStepText = isHttp
    ? 'Would you like to add custom headers?'
    : 'Would you like to add environment variables?';

  const lastStepInstruction = hasEnvVars
    ? `Add environment variable${envVars.length > 1 ? 's' : ''} for ${name}`
    : isHttp
    ? 'Choose No when asked to add custom headers'
    : 'Choose No when asked to add environment variables';

  return (
    <div>
      <ol>
        <li>Run the <code>configure</code> command:</li>
      </ol>
      <CodeBlock language="sh">{`goose configure`}</CodeBlock>

      <ol start={2}>
        <li>
          Choose to add a{' '}
          <code>{isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}</code>.
        </li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension 
│
◆  What type of extension would you like to add?
│  ○ Built-in Extension
${
  isHttp
    ? '│  ● Remote Extension (Streaming HTTP)\n│  ○ Command-line Extension (Run a local command or script)'
    : '│  ○ Remote Extension\n│  ● Command-line Extension (Run a local command or script)'
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
│  ${isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
// highlight-start
◆  What would you like to call this extension?
│  ${name}
// highlight-end
└`}</CodeBlock>

      {isHttp ? (
        <>
          <ol start={4}>
            <li>Enter the Streaming HTTP endpoint URI.</li>
          </ol>
          <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension 
│
◇  What type of extension would you like to add?
│  Remote Extension (Streaming HTTP)
│
◇  What would you like to call this extension?
│  ${name}
│
// highlight-start
◆  What is the Streaming HTTP endpoint URI?
│  ${url}
// highlight-end
└`}</CodeBlock>
        </>
      ) : (
        <>
          <ol start={4}>
            <li>Enter the command to run when this extension is used.</li>
          </ol>
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
│  ${isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
◇  What would you like to call this extension?
│  ${name}
│
${
  isHttp
    ? `◇  What is the Streaming HTTP endpoint URI?\n│  ${url}\n│`
    : `◇  What command should be run?\n│  ${command}\n│`
}
// highlight-start
◆  Please set the timeout for this tool (in secs):
│  ${timeout}
// highlight-end
└`}</CodeBlock>

      <ol start={6}>
        <li>Choose to add a description. If you select <code>No</code>, Goose will skip it.</li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension
│
◇  What type of extension would you like to add?
│  ${isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
◇  What would you like to call this extension?
│  ${name}
│
${
  isHttp
    ? `◇  What is the Streaming HTTP endpoint URI?\n│  ${url}\n│`
    : `◇  What command should be run?\n│  ${command}\n│`
}
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
// highlight-start
◆  Would you like to add a description?
│  No
// highlight-end
└`}</CodeBlock>

      <ol start={7}>
        <li>{lastStepInstruction}</li>
      </ol>

      {!hasEnvVars && (
        <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension 
│
◇  What type of extension would you like to add?
│  ${isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
◇  What would you like to call this extension?
│  ${name}
│
${
  isHttp
    ? `◇  What is the Streaming HTTP endpoint URI?\n│  ${url}\n│`
    : `◇  What command should be run?\n│  ${command}\n│`
}
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
◇  Would you like to add a description?
│  No
│
// highlight-start
◆  ${lastStepText}
│  No
// highlight-end
└  Added ${name} extension`}</CodeBlock>
      )}

      {hasEnvVars && (
        <>
          {infoNote && <div className="alert alert--info">{infoNote}</div>}

          <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension
│
◇  What type of extension would you like to add?
│  ${isHttp ? 'Remote Extension (Streaming HTTP)' : 'Command-line Extension'}
│
◇  What would you like to call this extension?
│  ${name}
│
${
  isHttp
    ? `◇  What is the Streaming HTTP endpoint URI?\n│  ${url}\n│`
    : `◇  What command should be run?\n│  ${command}\n│`
}
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
◇  Would you like to add a description?
│  No
│
// highlight-start
◆  ${lastStepText}
│  Yes
${envVars
  .map(
    ({ key, value }, i) => `│
◇  Environment variable name:
│  ${key}
│
◇  Environment variable value:
│  ${value}
│
◇  Add another environment variable?
│  ${i === envVars.length - 1 ? 'No' : 'Yes'}`
  )
  .join('\n')}
// highlight-end
└  Added ${name} extension`}</CodeBlock>
        </>
      )}
    </div>
  );
}
