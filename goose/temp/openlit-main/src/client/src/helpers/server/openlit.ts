import { encodingForModel, TiktokenModel } from "js-tiktoken";

export default class OpenLitHelper {
	static readonly PROMPT_TOKEN_FACTOR = 1000;
	static pricingInfo: any;

	static openaiTokens(text: string, model: string): number {
		try {
			const encoding = encodingForModel(model as TiktokenModel);
			return encoding.encode(text).length;
		} catch (error) {
			console.error(`Error in openaiTokens: ${error}`);
			throw error;
		}
	}

	static generalTokens(text: string): number {
		const encoding = encodingForModel("gpt2");
		return encoding.encode(text).length;
	}

	static getChatModelCost(
		model: string,
		promptTokens: number,
		completionTokens: number
	): string {
		try {
			return (
				(promptTokens / OpenLitHelper.PROMPT_TOKEN_FACTOR) *
					this.pricingInfo.chat[model].promptPrice +
				(completionTokens / OpenLitHelper.PROMPT_TOKEN_FACTOR) *
					this.pricingInfo.chat[model].completionPrice
			).toFixed(8);
		} catch (error) {
			console.error(`Error in getChatModelCost: ${error}`);
			return "0";
		}
	}

	static getEmbedModelCost(model: string, promptTokens: number): number {
		try {
			return (
				(promptTokens / OpenLitHelper.PROMPT_TOKEN_FACTOR) *
				this.pricingInfo.embeddings[model]
			);
		} catch (error) {
			console.error(`Error in getEmbedModelCost: ${error}`);
			return 0;
		}
	}

	static getImageModelCost(
		model: string,
		size: string,
		quality: number
	): number {
		try {
			return this.pricingInfo.images[model][quality][size];
		} catch (error) {
			console.error(`Error in getImageModelCost: ${error}`);
			return 0;
		}
	}

	static getAudioModelCost(model: string, prompt: string): number {
		try {
			return (
				(prompt.length / OpenLitHelper.PROMPT_TOKEN_FACTOR) *
				this.pricingInfo.audio[model]
			);
		} catch (error) {
			console.error(`Error in getAudioModelCost: ${error}`);
			return 0;
		}
	}

	static async fetchPricingInfo() {
		const pricingUrl =
			"https://raw.githubusercontent.com/openlit/openlit/main/assets/pricing.json";
		try {
			const response = await fetch(pricingUrl);
			if (response.ok) {
				this.pricingInfo = await response.json();
			} else {
				throw new Error(
					`HTTP error occurred while fetching pricing info: ${response.status}`
				);
			}
		} catch (error) {
			console.error(
				`Unexpected error occurred while fetching pricing info: ${error}`
			);
			return {};
		}
	}
}
