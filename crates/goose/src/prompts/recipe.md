Based on our conversation so far, could you create:

1. A concise set of instructions (1-2 paragraphs) that describe what you've been helping with. Make the instructions generic, and higher-level so that can be re-used across various similar tasks. Pay special attention if any output styles or formats are requested (and make it clear), and note any non standard tools used or required.

2. A list of 3-5 example activities (as a few words each at most) that would be relevant to this topic

Format your response in _VALID_ json, with one key being `instructions` which contains a string, and the other key `activities` as an array of strings.
For example, perhaps we have been discussing fruit and you might write:

{
"instructions": "Using web searches we find pictures of fruit, and always check what language to reply in.",
"activities": [
"Show pics of apples",
"say a random fruit",
"share a fruit fact"
]
}
