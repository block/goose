"use client";
import React, { memo } from 'react';
import { Card, CardContent, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import CodeBlock from "@/components/common/code-block";
import getMessage from '@/constants/messages';

export default memo(function TracingGettingStarted() {
  return (
    <div className="flex flex-col items-center p-8 w-full">
      <div className="w-full">
        <h1 className="text-3xl font-semibold text-center mb-4 text-stone-800 dark:text-stone-200">
          {getMessage().GET_STARTED_WITH_TRACING}
        </h1>
        <p className="text-center text-md mb-8 text-stone-500 dark:text-stone-500">
          {getMessage().GET_STARTED_WITH_TRACING_DESCRIPTION}
        </p>

        <div className='grid grid-cols-2 gap-6'>

          {/* Setup Instructions */}
          <div className="mb-12">
            <h2 className="text-2xl font-semibold mb-6 text-stone-800 dark:text-stone-200">
              Setup OpenLIT SDK
            </h2>

            <Tabs className="w-full" defaultValue="python">
              <TabsList>
                <TabsTrigger value="python">Python</TabsTrigger>
                <TabsTrigger value="typescript">TypeScript</TabsTrigger>
              </TabsList>

              {/* Python Tab */}
              <TabsContent value="python" className="space-y-6">
                <div>
                  <h3 className="text-lg font-semibold mb-3 text-primary">1. Install OpenLIT SDK</h3>
                  <p className="mb-2 text-sm text-stone-600 dark:text-stone-400">
                    Install the{" "}
                    <a
                      className="text-primary italic px-1"
                      href="https://github.com/openlit/openlit/tree/main/sdk/python"
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      OpenLIT Python SDK
                    </a>{" "}
                    using pip:
                  </p>
                  <CodeBlock
                    className="text-xs"
                    code={`pip install openlit`}
                    language="bash"
                  />
                </div>

                <div>
                  <h3 className="text-lg font-semibold mb-3 text-primary">2. Initialize OpenLIT in Your Application</h3>
                  <p className="mb-2 text-sm text-stone-600 dark:text-stone-400">
                    Add the following two lines to your application code:
                  </p>
                  <CodeBlock
                    className="text-xs"
                    code={`import openlit

openlit.init(otlp_endpoint="http://127.0.0.1:4318")`}
                    language="python"
                  />
                </div>

                <div>
                  <h3 className="text-lg font-semibold mb-3 text-primary">3. Example Usage with OpenAI</h3>
                  <CodeBlock
                    className="text-xs"
                    code={`from openai import OpenAI
import openlit

openlit.init(otlp_endpoint="http://127.0.0.1:4318")

client = OpenAI(api_key="YOUR_OPENAI_KEY")

chat_completion = client.chat.completions.create(
    messages=[
        {
            "role": "user",
            "content": "What is LLM Observability?",
        }
    ],
    model="gpt-3.5-turbo",
)`}
                    language="python"
                  />
                </div>

                <div className="bg-stone-50 dark:bg-stone-900 p-4 rounded-lg border border-stone-200 dark:border-stone-700">
                  <p className="text-sm text-stone-600 dark:text-stone-400">
                    <strong className="text-stone-800 dark:text-stone-200">💡 Tip:</strong> You can also set the endpoint using the{" "}
                    <code className="text-primary px-1">OTEL_EXPORTER_OTLP_ENDPOINT</code> environment variable.
                  </p>
                </div>
              </TabsContent>

              {/* TypeScript Tab */}
              <TabsContent value="typescript" className="space-y-6">
                <div>
                  <h3 className="text-lg font-semibold mb-3 text-primary">1. Install OpenLIT SDK</h3>
                  <p className="mb-2 text-sm text-stone-600 dark:text-stone-400">
                    Install the{" "}
                    <a
                      className="text-primary italic px-1"
                      href="https://github.com/openlit/openlit/tree/main/sdk/typescript"
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      OpenLIT TypeScript SDK
                    </a>{" "}
                    using npm:
                  </p>
                  <CodeBlock
                    className="text-xs"
                    code={`npm install openlit`}
                    language="bash"
                  />
                </div>

                <div>
                  <h3 className="text-lg font-semibold mb-3 text-primary">2. Initialize OpenLIT in Your Application</h3>
                  <p className="mb-2 text-sm text-stone-600 dark:text-stone-400">
                    Add the following lines to your application code:
                  </p>
                  <CodeBlock
                    className="text-xs"
                    code={`import openlit from 'openlit';

openlit.init({
  otlpEndpoint: "http://127.0.0.1:4318"
});`}
                    language="typescript"
                  />
                </div>

                <div>
                  <h3 className="text-lg font-semibold mb-3 text-primary">3. Example Usage with OpenAI</h3>
                  <CodeBlock
                    className="text-xs"
                    code={`import OpenAI from 'openai';
import openlit from 'openlit';

openlit.init({ otlpEndpoint: "http://127.0.0.1:4318" });

const client = new OpenAI({
  apiKey: process.env.OPENAI_API_KEY
});

const chatCompletion = await client.chat.completions.create({
  messages: [{ role: 'user', content: 'What is LLM Observability?' }],
  model: 'gpt-3.5-turbo',
});`}
                    language="typescript"
                  />
                </div>

                <div className="bg-stone-50 dark:bg-stone-900 p-4 rounded-lg border border-stone-200 dark:border-stone-700">
                  <p className="text-sm text-stone-600 dark:text-stone-400">
                    <strong className="text-stone-800 dark:text-stone-200">💡 Tip:</strong> You can also set the endpoint using the{" "}
                    <code className="text-primary px-1">OTEL_EXPORTER_OTLP_ENDPOINT</code> environment variable.
                  </p>
                </div>
              </TabsContent>
            </Tabs>
          </div>

          {/* Features Grid */}
          <div className="grid grid-cols-1 gap-6">
            {getMessage().GET_STARTED_WITH_TRACING_FEATURE_DETAILS.map((item, index) => (
              <Card key={item.title} className='border'>
                <CardTitle className='p-6 gap-2 flex items-center text-stone-700 dark:text-stone-300'>
                  <span className="text-2xl">{item.icon}</span>
                  <span>{item.title}</span>
                </CardTitle>
                <CardContent>
                  <p className="text-stone-500 dark:text-stone-400">
                    {item.description}
                  </p>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>

        {/* Footer Links */}
        <div className="mt-8 text-center">
          <p className="text-sm text-stone-500 dark:text-stone-500">
            For more information, visit the{" "}
            <a
              href="https://docs.openlit.io"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary hover:underline"
            >
              OpenLIT Documentation
            </a>
          </p>
        </div>
      </div>
    </div>
  );
})
