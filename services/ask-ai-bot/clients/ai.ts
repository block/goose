import { openrouter as anthropic } from "@openrouter/ai-sdk-provider";

const modelName = process.env.AI_MODEL || "claude-sonnet-4-6";

export const model = anthropic(modelName);
