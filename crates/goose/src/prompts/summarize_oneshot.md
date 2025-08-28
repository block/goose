## Task Context
- An llm context limit was reached when a user was in a working session with an agent (you)
- Generate a version of the below messages with only the most verbose parts removed
- Include user requests, your responses, all technical content, and as much of the original context as possible
- This will be used to let the user continue the working session
- Use framing and tone knowing the content will be read an agent (you) on a next exchange to allow for continuation of the session

**Conversation History:**
{{ messages }}

Wrap reasoning in `<analysis>` tags:  
- Review conversation chronologically
- For each part, log:  
  - User goals and requests  
  - Your method and solution  
  - Key decisions and designs  
  - File names, code, signatures, errors, fixes  
- Highlight user feedback and revisions  
- Confirm completeness and accuracy  
- This summary will only be read by you so it is ok to make it much longer than a normal summary you would show to a human
- Do not exclude any information that might be important to continuing a session working with you

### Include the Following Sections:
1. **User Intent** – All goals and requests  
2. **Technical Concepts** – All discussed tools, methods  
3. **Files + Code** – Viewed/edited files, full code, change justifications  
4. **Errors + Fixes** – Bugs, resolutions, user-driven changes  
5. **Problem Solving** – Issues solved or in progress  
6. **User Messages** – All user messages including tool calls, but truncate long tool call arguments or results
7. **Pending Tasks** – All unresolved user requests  
8. **Current Work** – Active work at summary request time: filenames, code, alignment to latest instruction  
9. **Next Step** – *Include only if* directly continues user instruction  

> No new ideas unless user confirmed
