import React from 'react';
import Layout from '@docusaurus/theme-classic/lib/theme/Layout';
import CodeBlock from '@docusaurus/theme-classic/lib/theme/CodeBlock';

const Types: React.FC = () => {
  return (
    <Layout title="Types" description="Type definitions for the Prompt Library">
      <div className="container margin-vert--lg">
        <h1>Type Definitions</h1>
        <p>This page contains the type definitions used in the Prompt Library.</p>
        
        <h2>Environment Variable</h2>
        <CodeBlock language="typescript">
{`type EnvironmentVariable = {
  name: string;
  description: string;
  required: boolean;
};`}
        </CodeBlock>

        <h2>Extension</h2>
        <CodeBlock language="typescript">
{`type Extension = {
  name: string;
  command: string;
  is_builtin: boolean;
  link?: string;
  installation_notes?: string;
  environmentVariables?: EnvironmentVariable[];
};`}
        </CodeBlock>

        <h2>Category</h2>
        <CodeBlock language="typescript">
{`type Category = "business" | "technical" | "productivity";`}
        </CodeBlock>

        <h2>Prompt</h2>
        <CodeBlock language="typescript">
{`type Prompt = {
  id: string;
  title: string;
  description: string;
  example_prompt: string;
  extensions: Extension[];
  category: Category;
  featured?: boolean;
};`}
        </CodeBlock>
      </div>
    </Layout>
  );
};

export default Types;