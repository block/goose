import React from 'react';
import CodeBlock from '@theme/CodeBlock';

export default function CLIExtensionInstructions({
  name,
  command,
  timeout = 300,
  envVars = [],
  infoNote,
}) {
  const hasEnvVars = envVars.length > 0;
  const envStepText = hasEnvVars
    ? `Add environment variable${envVars.length > 1 ? 's' : ''} for ${name}`
    : 'Choose No when asked to add environment variables';

  return (
    <div>
      <ol>
        <li>Run the <code>configure</code> command:</li>
      </ol>
      <CodeBlock language="sh">{`goose configure`}</CodeBlock>

      <ol start={2}>
        <li>Choose to add a <code>Command-line Extension</code>.</li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension (Connect to a new extension)
│
◆  What type of extension would you like to add?
│  ○ Built-in Extension 
// highlight-start    
│  ● Command-line Extension (Run a local command or script)
// highlight-end  
│  ○ Remote Extension 
└`}</CodeBlock>

      <ol start={3}>
        <li>Give your extension a name.</li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension (Connect to a new extension)
│
◇  What type of extension would you like to add?
│  Command-line Extension 
// highlight-start
◆  What would you like to call this extension?
│  ${name}
// highlight-end
└`}</CodeBlock>

      <ol start={4}>
        <li>Enter the command to run when this extension is used.</li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension (Connect to a new extension)
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

      <ol start={5}>
        <li>
          Enter the number of seconds Goose should wait for actions to complete before timing out. Default is <code>300</code> seconds.
        </li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension (Connect to a new extension) 
│
◇  What type of extension would you like to add?
│  Command-line Extension 
│
◇  What would you like to call this extension?
│  ${name}
│
◇  What command should be run?
│  ${command}
│
// highlight-start
◆  Please set the timeout for this tool (in secs):
│  ${timeout}
// highlight-end
└`}</CodeBlock>

      <ol start={6}>
        <li>Choose to add a description. If you select <code>Yes</code>, you’ll be prompted to enter a description for the extension.</li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension (Connect to a new extension)
│
◇  What type of extension would you like to add?
│  Command-line Extension 
│
◇  What would you like to call this extension?
│  ${name}
│
◇  What command should be run?
│  ${command}
│
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
// highlight-start
◆  Would you like to add a description?
│  No
// highlight-end
└`}</CodeBlock>

      <ol start={7}>
        <li>{envStepText}</li>
      </ol>

      {!hasEnvVars && (
        <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension (Connect to a new extension) 
│
◇  What type of extension would you like to add?
│  Command-line Extension 
│
◇  What would you like to call this extension?
│  ${name}
│
◇  What command should be run?
│  ${command}
│
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
◇  Would you like to add a description?
│  No
│
// highlight-start
◆  Would you like to add environment variables?
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
│  Add Extension (Connect to a new extension)
│
◇  What type of extension would you like to add?
│  Command-line Extension 
│
◇  What would you like to call this extension?
│  ${name}
│
◇  What command should be run?
│  ${command}
│
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
◇  Would you like to add a description?
│  No
│
// highlight-start
◆  Would you like to add environment variables?
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
