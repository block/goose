import { openrouter } from "@openrouter/ai-sdk-provider";

const modelName = process.env.AI_MODEL || "google/gemini-3-flash-preview";

export const model = openrouter(modelName);
