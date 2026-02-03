import { PromptCompiledInput, PromptInput } from "@/constants/prompts";

export function verifyPromptInput(promptInput: PromptInput) {
	if (promptInput.name.length === 0) {
		return {
			success: false,
			err: "Name should be present!",
		};
	}

	if (promptInput.prompt.length === 0) {
		return {
			success: false,
			err: "Prompt should be present!",
		};
	}

	return { success: true };
}

export function validatePromptCompiledInput(
	promptCompiledInput: PromptCompiledInput
) {
	if (!promptCompiledInput.apiKey) {
		return {
			success: false,
			err: "API key should be present!",
		};
	}

	if (!promptCompiledInput.id && !promptCompiledInput.name) {
		return {
			success: false,
			err: "Both Id and Name cannot be undefined!",
		};
	}

	return { success: true };
}
