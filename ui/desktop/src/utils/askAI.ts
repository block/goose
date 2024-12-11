import { getApiUrl, getSecretKey } from '../config';

const getQuestionClassifierPrompt = (messageContent: string): string => 
  `You are a simple classifier that takes content and decides if it is asking for input from a person before continuing if there is more to do, or not. These are questions on if a course of action should proceeed or not, or approval is needed. If it is a question very clearly, return QUESTION, otherwise READY. If it of the form of 'anything else I can do?' sort of question, return READY as that is not the sort of question we are looking for. ### Message Content:\n${messageContent}\nYou must provide a response strictly limited to one of the following two words: QUESTION, READY. No other words, phrases, or explanations are allowed. Response:`;

const getOptionsClassifierPrompt = (messageContent: string): string => 
  `You are a simple classifier that takes content and decides if it a list of options or plans to choose from, or not a list of options to choose from It is IMPORTANT that you really know this is a choice, just not numbered steps. If it is a list of options and you are 95% sure, return OPTIONS, otherwise return NO. ### Message Content:\n${messageContent}\nYou must provide a response strictly limited to one of the following two words:OPTIONS, NO. No other words, phrases, or explanations are allowed. Response:`;

const getOptionsFormatterPrompt = (messageContent: string): string => 
  `If the content is list of distinct options or plans of action to choose from, and not just a list of things, but clearly a list of things to choose one from, taking into account the Message Content alone, try to format it in a json array, like this JSON array of objects of the form optionTitle:string, optionDescription:string (markdown).\n If is not a list of options or plans to choose from, then return empty list.\n ### Message Content:\n${messageContent}\n\nYou must provide a response strictly as json in the format descriribed. No other words, phrases, or explanations are allowed. Response:`;

export const getPromptTemplates = (messageContent: string): string[] => [
  getQuestionClassifierPrompt(messageContent),
  getOptionsClassifierPrompt(messageContent),
  getOptionsFormatterPrompt(messageContent)
];

/**
 * Utility to ask the LLM any question to clarify without wider context.
 */
export async function askAi(promptTemplates: string[]) {
  const responses = await Promise.all(
    promptTemplates.map(async (template) => {
      const response = await fetch(getApiUrl('/ask'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({ prompt: template }),
      });

      if (!response.ok) {
        throw new Error('Failed to get response');
      }

      const data = await response.json();

      return data.response;
    })
  );

  return responses;
}