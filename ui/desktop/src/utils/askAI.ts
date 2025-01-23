import { getApiUrl, getSecretKey } from '../config';

const getQuestionClassifierPrompt = (messageContent: string): string => `
You are a simple classifier that takes content and decides if it is asking for input 
from a person before continuing if there is more to do, or not. These are questions 
on if a course of action should proceeed or not, or approval is needed. If it is CLEARLY a 
question asking if it ok to proceed or make a choice or some input is required to proceed, then, and ONLY THEN, return QUESTION, otherwise READY if not 97% sure.

### Examples message content that is classified as READY:
anything else I can do?
Could you please run the application and verify that the headlines are now visible in dark mode? You can use npm start.
Would you like me to make any adjustments to the formatting of these multiline strings?
Would you like me to show you how to ... (do something)?
Listing window titles... Is there anything specific you'd like help with using these tools?
Would you like me to demonstrate any specific capability or help you with a particular task?
Would you like me to run any tests?
Would you like me to make any adjustments or would you like to test?
Would you like me to dive deeper into any aspect?
Would you like me to make any other adjustments to this implementation?
Would you like any further information or assistance?
Would you like to me to make any changes?
Would you like me to make any adjustments to this implementation?
Would you like me to show you how to…
What would you like to do next?

### Examples that are QUESTIONS:
Should I go ahead and make the changes?
Should I Go ahead with this plan?
Should I focus on X or Y?
Provide me with the name of the package and version you would like to install.


### Message Content:
${messageContent}

You must provide a response strictly limited to one of the following two words: 
QUESTION, READY. No other words, phrases, or explanations are allowed.

Response:`;

const getOptionsClassifierPrompt = (messageContent: string): string => `
You are a simple classifier that takes content and decides if it a list of options 
or plans to choose from, or not a list of options to choose from. It is IMPORTANT 
that you really know this is a choice, just not numbered steps. If it is a list 
of options and you are 95% sure, return OPTIONS, otherwise return NO.

### Example (text -> response):
Would you like me to proceed with creating this file? Please let me know if you want any changes before I write it. -> NO
Here are some options for you to choose from: -> OPTIONS
which one do you want to choose? -> OPTIONS
Would you like me to dive deeper into any aspects of these components? -> NO
Should I focus on X or Y? -> OPTIONS

### Message Content:
${messageContent}

You must provide a response strictly limited to one of the following two words:
OPTIONS, NO. No other words, phrases, or explanations are allowed.

Response:`;

const getOptionsFormatterPrompt = (messageContent: string): string => `
If the content is list of distinct options or plans of action to choose from, and 
not just a list of things, but clearly a list of things to choose one from, taking 
into account the Message Content alone, try to format it in a json array, like this 
JSON array of objects of the form optionTitle:string, optionDescription:string (markdown).

If is not a list of options or plans to choose from, then return empty list.

### Message Content:
${messageContent}

You must provide a response strictly as json in the format descriribed. No other 
words, phrases, or explanations are allowed.

Response:`;

const getFormPrompt = (messageContent: string): string => `
When you see a request for several pieces of information, then provide a well formed JSON object like will be shown below.
The response will have:
* a title, description,
* a list of fields, each field will have a label, type, name, placeholder, and required (boolean).
(type is either text or textarea only).
If it is not requesting clearly several pieces of information, just return an empty object.
If the task could be confirmed without more information, return an empty object.

### Example Message:
I'll help you scaffold out a Python package. To create a well-structured Python package, I'll need to know a few key pieces of information:

Package name - What would you like to call your package? (This should be a valid Python package name - lowercase, no spaces, typically using underscores for separators if needed)

Brief description - What is the main purpose of the package? This helps in setting up appropriate documentation and structure.

Initial modules - Do you have specific functionality in mind that should be split into different modules?

Python version - Which Python version(s) do you want to support?

Dependencies - Are there any known external packages you'll need?

### Example JSON Response:
{
  "title": "Python Package Scaffolding Form",
  "description": "Provide the details below to scaffold a well-structured Python package.",
  "fields": [
    {
      "label": "Package Name",
      "type": "text",
      "name": "package_name",
      "placeholder": "Enter the package name (lowercase, no spaces, use underscores if needed)",
      "required": true
    },
    {
      "label": "Brief Description",
      "type": "textarea",
      "name": "brief_description",
      "placeholder": "Enter a brief description of the package's purpose",
      "required": true
    },
    {
      "label": "Initial Modules",
      "type": "textarea",
      "name": "initial_modules",
      "placeholder": "List the specific functionalities or modules (optional)",
      "required": false
    },
    {
      "label": "Python Version(s)",
      "type": "text",
      "name": "python_versions",
      "placeholder": "Enter the Python version(s) to support (e.g., 3.8, 3.9, 3.10)",
      "required": true
    },
    {
      "label": "Dependencies",
      "type": "textarea",
      "name": "dependencies",
      "placeholder": "List any known external packages you'll need (optional)",
      "required": false
    }
  ]
}

### Message Content:
${messageContent}

You must provide a response strictly as json in the format described. No other 
words, phrases, or explanations are allowed.

Response:`;

/**
 * Core function to ask the AI a single question and get a response
 * @param prompt The prompt to send to the AI
 * @returns Promise<string> The AI's response
 */
export async function ask(prompt: string): Promise<string> {
  const response = await fetch(getApiUrl('/ask'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({ prompt }),
  });

  if (!response.ok) {
    throw new Error('Failed to get response');
  }

  const data = await response.json();
  return data.response;
}

/**
 * Utility to ask the LLM multiple questions to clarify without wider context.
 * @param messageContent The content to analyze
 * @returns Promise<string[]> Array of responses from the AI for each classifier
 */
export async function askAi(messageContent: string): Promise<string[]> {
  // First, check the question classifier
  const questionClassification = await ask(getQuestionClassifierPrompt(messageContent));

  // If READY, return early with empty responses for options
  if (questionClassification === 'READY') {
    return [questionClassification, 'NO', '[]', '{}'];
  }

  // Otherwise, proceed with all classifiers in parallel
  const prompts = [
    Promise.resolve(questionClassification), // Reuse the result we already have
    ask(getOptionsClassifierPrompt(messageContent)),
    ask(getOptionsFormatterPrompt(messageContent)),
    ask(getFormPrompt(messageContent)),
  ];

  return Promise.all(prompts);
}
