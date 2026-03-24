import { anthropic } from "@ai-sdk/anthropic";

const modelName = process.env.AI_MODEL || "claude-sonnet-4-6";

export const model = anthropic(modelName);
