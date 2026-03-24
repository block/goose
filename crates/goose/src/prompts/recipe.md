Based on our conversation so far, could you create:

1. A concise title (5-10 words) that captures the main topic or task
2. A brief description (1-2 sentences) that summarizes what this recipe helps with
3. A concise set of instructions (1-2 paragraphs) that describe what you've been helping with. Make the instructions generic, and higher-level so that can be re-used across various similar tasks. Pay special attention if any output styles or formats are requested (and make it clear), and note any non standard tools used or required.
4. A list of 3-5 example activities (as a few words each at most) that would be relevant to this topic

Format your response in _VALID_ json, with keys being `title`, `description`, `instructions` (string), and `activities` (array of strings).
For example, perhaps we have been discussing fruit and you might write:

{
"title": "Fruit Information Assistant",
"description": "A recipe for finding and sharing information about different types of fruit.",
"instructions": "Using web searches we find pictures of fruit, and always check what language to reply in.",
"activities": [
"Show pics of apples",
"say a random fruit",
"share a fruit fact"
]
}
