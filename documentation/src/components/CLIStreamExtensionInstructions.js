import React from 'react';
import CodeBlock from '@theme/CodeBlock';
import Admonition from '@theme/Admonition';

export default function CLIStreamExtensionInstructions({
  name,
  endpointUri,
  timeout = 300,
  headers = [],
  infoNote,
}) {
  const hasHeaders = headers.length > 0;
  const headerStepText = hasHeaders
    ? `Choose Yes when asked to add custom headers`
    : 'Choose No when asked to add custom headers';

  return (
    <div>
      <ol>
        <li>Run the <code>configure</code> command:</li>
      </ol>
      <CodeBlock language="sh">{`goose configure`}</CodeBlock>

      <ol start={2}>
        <li>Choose to add a <code>Remote Extension (Streaming HTTP)</code></li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension
│
◆  What type of extension would you like to add?
│  ○ Built-in Extension 
│  ○ Command-line Extension
│  ○ Remote Extension (SSE)
// highlight-start    
│  ● Remote Extension (Streaming HTTP) (Connect to a remote extension via MCP Streaming HTTP)
// highlight-end  
└`}</CodeBlock>

      <ol start={3}>
        <li>Give your extension a name</li>
      </ol>
      <CodeBlock language="sh">{`┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension
│
◇  What type of extension would you like to add?
│  Remote Extension (Streaming HTTP)
│
// highlight-start
◆  What would you like to call this extension?
│  ${name}
// highlight-end
└`}</CodeBlock>

      <ol start={4}>
        <li>Enter the endpoint URI</li>
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
│  ${endpointUri}
// highlight-end
└`}</CodeBlock>

      <ol start={5}>
        <li>
          Enter the number of seconds Goose should wait for actions to complete before timing out. Default is <code>300</code> seconds
        </li>
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
◇  What is the Streaming HTTP endpoint URI?
│  ${endpointUri}
│
// highlight-start
◆  Please set the timeout for this tool (in secs):
│  ${timeout}
// highlight-end
└`}</CodeBlock>

      <ol start={6}>
        <li>Choose to add a description. If you select <code>Yes</code>, you'll be prompted to enter a description for the extension</li>
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
◇  What is the Streaming HTTP endpoint URI?
│  ${endpointUri}
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
        <li>{headerStepText}</li>
      </ol>

      {!hasHeaders && (
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
◇  What is the Streaming HTTP endpoint URI?
│  ${endpointUri}
│
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
◇  Would you like to add a description?
│  No
│
// highlight-start
◆  Would you like to add custom headers?
│  No
// highlight-end
└  Added ${name} extension`}</CodeBlock>
      )}

      {hasHeaders && (
        <>
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
◇  What is the Streaming HTTP endpoint URI?
│  ${endpointUri}
│
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
◇  Would you like to add a description?
│  No
│
// highlight-start
◆  Would you like to add custom headers?
│  Yes
// highlight-end
└`}</CodeBlock>

          <ol start={8}>
            <li>Add your custom header{headers.length > 1 ? 's' : ''}</li>
          </ol>

          {infoNote && (
            <>
              <Admonition type="info">
                {infoNote}
              </Admonition>
              <div style={{ marginBottom: '0.5rem' }} />
            </>
          )}

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
◇  What is the Streaming HTTP endpoint URI?
│  ${endpointUri}
│
◇  Please set the timeout for this tool (in secs):
│  ${timeout}
│
◇  Would you like to add a description?
│  No
│
◇  Would you like to add custom headers?
│  Yes
│
// highlight-start
${headers
  .map(
    ({ key, value }, i) => `◇  Header name:
│  ${key}
│
◇  Header value:
│  ${value}
│
◇  Add another header?
│  ${i === headers.length - 1 ? 'No' : 'Yes'}`
  )
  .join('\n│\n')}
// highlight-end
└  Added ${name} extension`}</CodeBlock>
        </>
      )}
    </div>
  );
}
