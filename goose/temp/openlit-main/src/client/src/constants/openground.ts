import { ProviderType, Providers } from "@/types/store/openground";

export const providersConfig: Record<Providers, ProviderType> = {
	openai: {
		key: "openai",
		title: "OpenAI",
		subTitle: "Chat completions",
		logoDark: "/images/provider/openai.png",
		logo: "/images/provider/openai-white.png",
		config: [
			{
				key: "type",
				label: "Type",
				type: "hidden",
				defaultValue: "chat",
			},
			{
				key: "api_key",
				label: "API Key",
				type: "input",
				placeholder: "OPENAI_API_KEY",
			},
			{
				key: "model",
				label: "Model",
				type: "select",
				placeholder: "select a model",
				defaultValue: "gpt-3.5-turbo",
				options: [
					{
						label: "gpt-3.5-turbo",
						value: "gpt-3.5-turbo",
					},
					{
						label: "gpt-3.5-turbo-0125",
						value: "gpt-3.5-turbo-0125",
					},
					{
						label: "gpt-4",
						value: "gpt-4",
					},
					{
						label: "gpt-4-1106-preview",
						value: "gpt-4-1106-preview",
					},
					{
						label: "gpt-4-1106-vision-preview",
						value: "gpt-4-1106-vision-preview",
					},
					{
						label: "gpt-4-0125-preview",
						value: "gpt-4-0125-preview",
					},
					{
						label: "gpt-4-32k",
						value: "gpt-4-32k",
					},
					{
						label: "gpt-4-vision-preview",
						value: "gpt-4-vision-preview",
					},
				],
			},
			{
				key: "temperature",
				label: "Temperature",
				type: "slider",
				limits: { min: 0, max: 2, step: 0.1 },
				defaultValue: 1,
			},
			{
				key: "max_tokens",
				label: "Max tokens",
				type: "slider",
				limits: { min: 0, max: 4000, step: 1 },
				defaultValue: 100,
			},
			{
				key: "top_p",
				label: "Top P",
				type: "slider",
				limits: { min: 0, max: 1, step: 0.1 },
				defaultValue: 0.5,
			},
		],
	},
	anthropic: {
		key: "anthropic",
		title: "Anthropic",
		subTitle: "Create message",
		logoDark: "/images/provider/anthropic.png",
		logo: "/images/provider/anthropic-white.png",
		config: [
			{
				key: "type",
				label: "Type",
				type: "hidden",
				defaultValue: "message",
			},
			{
				key: "api_key",
				label: "API Key",
				type: "input",
				placeholder: "ANTHROPIC_API_KEY",
			},
			{
				key: "model",
				label: "Model",
				type: "select",
				placeholder: "select a model",
				defaultValue: "claude-3-opus-20240229",
				options: [
					{
						label: "Claude 3 Opus",
						value: "claude-3-opus-20240229",
					},
					{
						label: "Claude 3 Sonnet",
						value: "claude-3-sonnet-20240229",
					},
					{
						label: "Claude 3 Haiku",
						value: "claude-3-haiku-20240307",
					},
				],
			},
			{
				key: "max_tokens",
				label: "Max tokens",
				type: "slider",
				limits: { min: 0, max: 4000, step: 1 },
				defaultValue: 100,
			},
		],
	},
	cohere: {
		key: "cohere",
		title: "Cohere",
		subTitle: "Chat completion",
		logoDark: "/images/provider/cohere.png",
		logo: "/images/provider/cohere-white.png",
		config: [
			{
				key: "type",
				label: "Type",
				type: "hidden",
				defaultValue: "chat",
			},
			{
				key: "token",
				label: "Token",
				type: "input",
				placeholder: "COHERE_TOKEN",
			},
			{
				key: "model",
				label: "Model",
				type: "select",
				placeholder: "select a model",
				defaultValue: "command",
				options: [
					{
						label: "Command",
						value: "command",
					},
					{
						label: "Command Nightly",
						value: "command-nightly",
					},
					{
						label: "Command Light",
						value: "command-light",
					},
					{
						label: "Command Light Nightly",
						value: "command-light-nightly",
					},
				],
			},
			{
				key: "max_tokens",
				label: "Max tokens",
				type: "slider",
				limits: { min: 0, max: 4000, step: 1 },
				defaultValue: 100,
			},
			{
				key: "p",
				label: "P",
				type: "slider",
				limits: { min: 0.01, max: 0.99, step: 0.01 },
				defaultValue: 0.75,
			},
		],
	},
	mistral: {
		key: "mistral",
		title: "Mistral",
		subTitle: "Chat completion",
		logoDark: "/images/provider/mistral.png",
		logo: "/images/provider/mistral-white.png",
		config: [
			{
				key: "type",
				label: "Type",
				type: "hidden",
				defaultValue: "chat",
			},
			{
				key: "api_key",
				label: "API Key",
				type: "input",
				placeholder: "MISTRAL_API_KEY",
			},
			{
				key: "model",
				label: "Model",
				type: "select",
				placeholder: "select a model",
				defaultValue: "open-mistral-7b",
				options: [
					{
						label: "Mistral 7B",
						value: "open-mistral-7b",
					},
					{
						label: "Mixtral 8x7B",
						value: "open-mixtral-8x7b",
					},
					{
						label: "Mistral Small",
						value: "mistral-small-latest",
					},
					{
						label: "Mistral Large",
						value: "mistral-large-latest",
					},
				],
			},
			{
				key: "temperature",
				label: "Temperature",
				type: "slider",
				limits: { min: 0, max: 1, step: 0.1 },
				defaultValue: 1,
			},
			{
				key: "max_tokens",
				label: "Max tokens",
				type: "slider",
				limits: { min: 0, max: 4000, step: 1 },
				defaultValue: 100,
			},
			{
				key: "top_p",
				label: "Top P",
				type: "slider",
				limits: { min: 0, max: 1, step: 0.01 },
				defaultValue: 0.5,
			},
		],
	},
	google: {
		key: "google",
		title: "Google AI",
		subTitle: "Gemini models",
		logoDark: "/images/provider/google.png",
		logo: "/images/provider/google-white.png",
		config: [
			{
				key: "type",
				label: "Type",
				type: "hidden",
				defaultValue: "chat",
			},
			{
				key: "api_key",
				label: "API Key",
				type: "input",
				placeholder: "GOOGLE_API_KEY",
			},
			{
				key: "model",
				label: "Model",
				type: "select",
				placeholder: "select a model",
				defaultValue: "gemini-1.5-pro",
				options: [
					{
						label: "Gemini 1.5 Pro",
						value: "gemini-1.5-pro",
					},
					{
						label: "Gemini 1.5 Flash",
						value: "gemini-1.5-flash",
					},
					{
						label: "Gemini Pro",
						value: "gemini-pro",
					},
					{
						label: "Gemini Pro Vision",
						value: "gemini-pro-vision",
					},
				],
			},
			{
				key: "temperature",
				label: "Temperature",
				type: "slider",
				limits: { min: 0, max: 2, step: 0.1 },
				defaultValue: 1,
			},
			{
				key: "max_tokens",
				label: "Max tokens",
				type: "slider",
				limits: { min: 0, max: 8000, step: 1 },
				defaultValue: 1000,
			},
			{
				key: "top_p",
				label: "Top P",
				type: "slider",
				limits: { min: 0, max: 1, step: 0.1 },
				defaultValue: 0.95,
			},
		],
	},
};
