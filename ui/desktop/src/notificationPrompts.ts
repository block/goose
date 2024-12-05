/**
 * Prompt templates for classifying AI responses and extracting options
 */

/**
 * Template to determine if the AI response is asking for user input/approval
 */
export const inputRequiredTemplate = (content: string) => `You are a simple classifier that takes content and decides if it is asking for input from a person before continuing if there is more to do, or not. These are questions on if a course of action should proceeed or not, or approval is needed. If it is a question that is very specific and clear, return QUESTION, otherwise READY. If it of the form of 'anything else I can do?' sort of question, return READY as that is not the sort of question we are looking for

### Examples:
'Would you like me to help you with any specific task using these capabilities?' -> READY

### Message Content:
${content}

You must provide a response strictly limited to one of the following two words: QUESTION, READY. No other words, phrases, or explanations are allowed. Response:`;

/**
 * Template to determine if the AI response contains a list of options
 */
export const hasOptionsTemplate = (content: string) => `You are a simple classifier that takes content and decides if it a list of options or plans to choose from, or not a list of options to choose from It is IMPORTANT that you really know this is a choice, just not numbered steps. If it is a list of options and you are 97% sure, return OPTIONS, otherwise return NO.

### Message Content:
${content}

You must provide a response strictly limited to one of the following two words:OPTIONS, NO. No other words, phrases, or explanations are allowed. Response:`;

/**
 * Template to extract and format options from AI response
 */
export const formatOptionsTemplate = (content: string) => `If the content is list of distinct options or plans of action to choose from, and not just a list of things, but clearly a list of things to choose one from, taking into account the Message Content alone, try to format it in a json array, like this JSON array of objects of the form optionTitle:string, optionDescription:string (markdown).
If is not a list of options or plans to choose from, then return empty list.

### Message Content:
${content}

You must provide a response strictly as json in the format descriribed. No other words, phrases, or explanations are allowed. Response:`;

/**
 * Get all prompt templates for a given content
 */
export const getPromptTemplates = (content: string): string[] => [
    inputRequiredTemplate(content),
    hasOptionsTemplate(content),
    formatOptionsTemplate(content)
];

export default getPromptTemplates;