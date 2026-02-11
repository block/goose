import { stepCountIs, streamText } from "ai";
import type { Message, ThreadChannel } from "discord.js";
import { model } from "../../clients/ai";
import { logger } from "../logger";
import { chunkMarkdown } from "./chunk-markdown";
import { MAX_STEPS, SYSTEM_PROMPT } from "./system-prompt";
import { ToolTracker } from "./tool-tracker";
import { aiTools } from "./tools";

export interface MessageHistoryItem {
  author: string;
  content: string;
  isBot: boolean;
}

export interface AnswerQuestionOptions {
  question: string;
  thread: ThreadChannel;
  userId: string;
  messageHistory?: MessageHistoryItem[];
  statusMessage?: Message;
}

export async function answerQuestion({
  question,
  thread,
  userId,
  messageHistory,
  statusMessage,
}: AnswerQuestionOptions): Promise<void> {
  try {
    let prompt = question;
    if (messageHistory && messageHistory.length > 0) {
      const historyContext = messageHistory
        .slice(0, -1)
        .map((msg) => `${msg.author}: ${msg.content}`)
        .join("\n");

      if (historyContext) {
        prompt = `# Previous conversation\n${historyContext}\n\n# New message\n${messageHistory[messageHistory.length - 1].author}: ${question}`;
      }
    }

    const tracker = new ToolTracker();

    const result = streamText({
      model,
      system: SYSTEM_PROMPT,
      prompt,
      tools: aiTools,
      stopWhen: stepCountIs(MAX_STEPS),
    });

    for await (const event of result.fullStream) {
      if (event.type === "tool-call") {
        if (event.toolName === "search_docs" && statusMessage) {
          try {
            await statusMessage.edit("Searching the docs...");
          } catch (error) {
            logger.verbose("Failed to update status message:", error);
          }
        } else if (event.toolName === "view_docs" && statusMessage) {
          const input = event.input as { filePaths?: string | string[] };
          const filePaths = input.filePaths;
          const pathArray = Array.isArray(filePaths) ? filePaths : [filePaths];
          const pagesText = pathArray.length === 1 ? "page" : "pages";
          try {
            await statusMessage.edit(
              `Viewing ${pathArray.length} ${pagesText}...`,
            );
          } catch (error) {
            logger.verbose("Failed to update status message:", error);
          }
        }
      } else if (event.type === "tool-result") {
        if (event.toolName === "search_docs") {
          const resultText = String(event.output);
          const fileMatches = resultText.match(/\*\*[^*]+\*\*/g) || [];
          tracker.recordSearchCall(fileMatches.map((_, i) => `result_${i}`));
        } else if (event.toolName === "view_docs") {
          const input = event.input as { filePaths?: string | string[] };
          const filePaths = input.filePaths;
          const pathArray = Array.isArray(filePaths)
            ? filePaths
            : filePaths
              ? [filePaths]
              : [];
          if (pathArray.length > 0) {
            tracker.recordViewCall(pathArray);
          }
        }
      }
    }

    if (statusMessage) {
      try {
        const summary = tracker.getSummary();
        await statusMessage.edit(summary || "Just a sec...");
      } catch (error) {
        logger.verbose("Failed to update final status message:", error);
      }
    }

    const fullText = await result.text;
    const chunks = chunkMarkdown(fullText);
    for (const chunk of chunks) {
      await thread.send(chunk);
    }

    const totalUsage = await result.usage;
    const { totalTokens } = totalUsage;
    logger.verbose(
      `Answered question for user ${userId}, tokens: ${totalTokens}`,
    );
  } catch (error) {
    logger.error("Failed to answer question:", error);
    await thread.send(
      "Sorry, I encountered an error while researching your question. Please try again.",
    );
    throw error;
  }
}
