import { omit } from "lodash";
import Anthropic from "@anthropic-ai/sdk";
import OpenLitHelper from "@/helpers/server/openlit";

export interface AnthropicProviderEvaluation {
	api_key?: string;
	prompt: string;
	type: "message";
	model: string;
	max_tokens: number;
}

export default class AnthropicProvider {
	private static async createMessage(params: any) {
		const anthropic = new Anthropic({
			apiKey: params.api_key,
		});

		const [err, updatedParams] = await this.transformParams(params);
		if (err) throw new Error(err);

		return await anthropic.messages.create(
			omit(updatedParams, ["api_key"]) as any
		);
	}

	private static async transformParams(params: AnthropicProviderEvaluation) {
		const updatedParams: any = {};
		updatedParams.api_key = params.api_key || process.env.ANTHROPIC_API_KEY;
		updatedParams.model = params.model;
		updatedParams.messages = [{ role: "user", content: params.prompt }];
		updatedParams.max_tokens = params.max_tokens;
		updatedParams.stream = false;
		return [null, updatedParams];
	}

	static async evaluate(params: AnthropicProviderEvaluation) {
		switch (params.type) {
			case "message":
				const start = performance.now();
				const response = await this.createMessage(params);
				const end = performance.now();
				const updatedResponse: any = response;
				updatedResponse.evaluationData = {
					cost: OpenLitHelper.getChatModelCost(
						response.model,
						response.usage.input_tokens,
						response.usage.output_tokens
					),
					promptTokens: response.usage.input_tokens || 0,
					completionTokens: response.usage.output_tokens || 0,
					responseTime: ((end - start) / 1000).toFixed(4),
					model: params.model,
					prompt: params.prompt,
					response: response.content[0].text,
				};
				return updatedResponse;
			default:
				throw new Error("No supported operation");
		}
	}
}
