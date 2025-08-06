## Summary Task
Generate detailed summary of conversation to date.  
Include user requests, your responses, and all technical content.  

Wrap reasoning in `<analysis>` tags:  
- Review conversation chronologically  
- For each part, log:  
  - User goals and requests  
  - Your method and solution  
  - Key decisions and designs  
  - File names, code, signatures, errors, fixes  
- Highlight user feedback and revisions  
- Confirm completeness and accuracy  

### Summary Must Include the Following Sections:  
1. **User Intent** – All goals and requests  
2. **Technical Concepts** – All discussed tools, methods  
3. **Files + Code** – Viewed/edited files, full code, change justifications  
4. **Errors + Fixes** – Bugs, resolutions, user-driven changes  
5. **Problem Solving** – Issues solved or in progress  
6. **User Messages** – All user messages, exclude tool output  
7. **Pending Tasks** – All unresolved user requests  
8. **Current Work** – Active work at summary request time: filenames, code, alignment to latest instruction  
9. **Next Step** – *Include only if* directly continues user instruction  

> No new ideas unless user confirmed
