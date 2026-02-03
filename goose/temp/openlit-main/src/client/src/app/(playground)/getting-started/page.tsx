"use client";
import CodeBlock from "@/components/common/code-block";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

export default function GettingStarted() {
	const code = [
		{
			key: "python",
			text: "Python",
			component: (
				<div className="flex flex-col text-sm rounded-b-lg text-stone-950 dark:text-stone-100">
					<h3 className="text-lg font-semibold mb-3 text-primary">1. Deploy OpenLIT</h3>
					<p className="mb-2">
						Git Clone OpenLIT Repository:
					</p>
					<CodeBlock
						className="text-xs"
						code={`git clone git@github.com:openlit/openlit.git`}
						language="bash"
					/>
					<p className="mt-2 mb-2">
						Start Docker Compose from the root directory:
					</p>
					<CodeBlock
						className="text-xs"
						code={`docker compose up -d`}
						language="bash"
					/>

					<h3 className="text-lg font-semibold mb-3 mt-6 text-primary">2. Install OpenLIT SDK</h3>
					<p className="mb-2">
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

					<h3 className="text-lg font-semibold mb-3 mt-6 text-primary">3. Initialize OpenLIT in Your Application</h3>
					<p className="mb-2">
						Add the following two lines to your application code:
					</p>
					<CodeBlock
						className="text-xs"
						code={`import openlit

openlit.init(otlp_endpoint="http://127.0.0.1:4318")`}
						language="python"
					/>
					<p className="mt-2 mb-2">
						Alternatively, you can set the endpoint using the{" "}
						<span className="text-primary italic px-1">
							OTEL_EXPORTER_OTLP_ENDPOINT
						</span>{" "}
						environment variable:
					</p>
					<CodeBlock
						className="text-xs"
						code={`export OTEL_EXPORTER_OTLP_ENDPOINT="http://127.0.0.1:4318"`}
						language="bash"
					/>
					<CodeBlock
						className="text-xs"
						code={`import openlit

openlit.init()`}
						language="python"
					/>

					<h3 className="text-lg font-semibold mb-3 mt-6 text-primary">Example Usage</h3>
					<p className="mb-2">
						Example with{" "}
						<a
							className="text-primary italic px-1"
							href="https://platform.openai.com/docs/introduction"
							target="_blank"
							rel="noopener noreferrer"
						>
							OpenAI
						</a>:
					</p>
					<CodeBlock
						className="text-xs"
						code={`from openai import OpenAI
import openlit

openlit.init(otlp_endpoint="http://127.0.0.1:4318")

client = OpenAI(
    api_key="YOUR_OPENAI_KEY"
)

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

					<h3 className="text-lg font-semibold mb-3 mt-6 text-primary">4. Visualize and Analyze</h3>
					<p className="mb-2">
						Head over to OpenLIT at{" "}
						<a
							className="text-primary italic px-1"
							href="http://127.0.0.1:3000"
							target="_blank"
							rel="noopener noreferrer"
						>
							127.0.0.1:3000
						</a>{" "}
						to start exploring. Login using the default credentials:
					</p>
					<ul className="list-disc list-inside mb-4 space-y-1">
						<li><strong>Email:</strong> user@openlit.io</li>
						<li><strong>Password:</strong> openlituser</li>
					</ul>
					<p className="mt-2">
						Refer to the{" "}
						<a
							className="text-primary italic px-1"
							href="https://github.com/openlit/openlit/tree/main/sdk/python"
							target="_blank"
							rel="noopener noreferrer"
						>
							OpenLIT Python SDK
						</a>{" "}
						repository for more advanced configurations and use cases.
					</p>
				</div>
			),
		},
		{
			key: "typescript",
			text: "TypeScript",
			component: (
				<div className="flex flex-col text-sm rounded-b-lg text-stone-950 dark:text-stone-100">
					<h3 className="text-lg font-semibold mb-3 text-primary">1. Deploy OpenLIT</h3>
					<p className="mb-2">
						Git Clone OpenLIT Repository:
					</p>
					<CodeBlock
						className="text-xs"
						code={`git clone git@github.com:openlit/openlit.git`}
						language="bash"
					/>
					<p className="mt-2 mb-2">
						Start Docker Compose from the root directory:
					</p>
					<CodeBlock
						className="text-xs"
						code={`docker compose up -d`}
						language="bash"
					/>

					<h3 className="text-lg font-semibold mb-3 mt-6 text-primary">2. Install OpenLIT SDK</h3>
					<p className="mb-2">
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

					<h3 className="text-lg font-semibold mb-3 mt-6 text-primary">3. Initialize OpenLIT in Your Application</h3>
					<p className="mb-2">
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
					<p className="mt-2 mb-2">
						Alternatively, you can set the endpoint using the{" "}
						<span className="text-primary italic px-1">
							OTEL_EXPORTER_OTLP_ENDPOINT
						</span>{" "}
						environment variable:
					</p>
					<CodeBlock
						className="text-xs"
						code={`export OTEL_EXPORTER_OTLP_ENDPOINT="http://127.0.0.1:4318"`}
						language="bash"
					/>
					<CodeBlock
						className="text-xs"
						code={`import openlit from 'openlit';

openlit.init();`}
						language="typescript"
					/>

					<h3 className="text-lg font-semibold mb-3 mt-6 text-primary">4. Visualize and Analyze</h3>
					<p className="mb-2">
						Head over to OpenLIT at{" "}
						<a
							className="text-primary italic px-1"
							href="http://127.0.0.1:3000"
							target="_blank"
							rel="noopener noreferrer"
						>
							127.0.0.1:3000
						</a>{" "}
						to start exploring. Login using the default credentials:
					</p>
					<ul className="list-disc list-inside mb-4 space-y-1">
						<li><strong>Email:</strong> user@openlit.io</li>
						<li><strong>Password:</strong> openlituser</li>
					</ul>
					<p className="mt-2">
						Refer to the{" "}
						<a
							className="text-primary italic px-1"
							href="https://github.com/openlit/openlit/tree/main/sdk/typescript"
							target="_blank"
							rel="noopener noreferrer"
						>
							OpenLIT TypeScript SDK
						</a>{" "}
						repository for more advanced configurations and use cases.
					</p>
				</div>
			),
		},
	];

	return (
		<div className="flex flex-col w-full flex-1 overflow-auto relative bg-stone-100/50 dark:bg-stone-900/50 p-4 rounded-md">
			<div className="mb-6">
				<h1 className="text-2xl font-bold mb-4 text-stone-950 dark:text-stone-100">
					Get started with AI Observability
				</h1>
				<p className="mb-4 text-sm font-medium border-l-4 border-primary p-3 text-primary bg-red-50 dark:bg-red-950">
					OpenLIT is an OpenTelemetry-native GenAI and LLM Application
					Observability tool. It&apos;s designed to make the integration process
					of observability into GenAI projects as easy as pie – literally, with
					just a single line of code. Whether you&apos;re working with popular LLM
					Libraries such as OpenAI and HuggingFace or leveraging vector databases
					like ChromaDB, OpenLIT ensures your applications are monitored
					seamlessly, providing critical insights to improve performance and
					reliability.
				</p>
				<p className="mb-5 text-stone-950 dark:text-stone-100 text-sm">
					This guide will walk you through setting up{" "}
					<span className="text-primary">OpenTelemetry</span> Auto Instrumentation
					for monitoring your LLM Application using OpenLIT. In just a few steps, you&apos;ll be able
					to track and analyze the performance and usage of your{" "}
					<span className="text-primary">GenAI</span> and{" "}
					<span className="text-primary">LLM Applications</span>. In this guide,
					we&apos;ll show how you can send OpenTelemetry traces and metrics from your
					LLM Applications to OpenLIT.
				</p>
			</div>
			<Tabs className="w-full" defaultValue={code[0].key}>
				<TabsList>
					{code.map((item) => (
						<TabsTrigger key={item.key} value={item.key}>
							{item.text}
						</TabsTrigger>
					))}
				</TabsList>
				{code.map((item) => (
					<TabsContent key={item.key} value={item.key} className="w-full">
						{item.component}
					</TabsContent>
				))}
			</Tabs>
		</div>
	);
}
