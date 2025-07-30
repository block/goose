## 📝 Summary Generation Instructions

Your task is to generate a comprehensive summary of the conversation so far, with close attention to the user's explicit requests and your own prior actions.  
This summary must fully capture all technical details, code structures, and architectural decisions critical to resuming development work without losing context.

Before presenting the final summary, enclose your reasoning in `<analysis>` tags to organize your thought process and confirm that you've addressed all required components.  
During your analysis, follow this approach:

### 🔍 Analysis Process

- Review the conversation **chronologically**, section by section.  
- For each part, clearly identify:
  - ✅ The user’s **explicit requests** and stated **intentions**
  - 🛠️ Your **approach** and method for addressing those requests
  - 🧠 Major **technical decisions**, **concepts**, and **design choices**
  - 🧩 Specific technical elements such as:
    - `file names`
    - `complete code snippets`
    - `function signatures`
    - `code modifications`
    - `errors encountered` and how they were resolved

- 🔁 Pay special attention to **direct user feedback** — especially any revisions or corrections.
- 📋 Double-check for **technical completeness and accuracy**, ensuring that **every required element** has been thoroughly addressed.

## 📄 Required Sections in Your Summary

### 1. **Primary Request and Intent**  
Capture all of the user’s **core goals** and **specific requests** throughout the conversation.

### 2. **Key Technical Concepts**  
List all major **technical concepts**, **technologies**, **tools**, or **frameworks** discussed.

### 3. **Files and Code Sections**  
Detail the specific **files and code regions** that were viewed, changed, or created.  
Include **full code snippets** where relevant, and explain **why** each change or file mattered.

### 4. **Errors and Fixes**  
List all **errors**, bugs, or unexpected behavior you encountered — and how you resolved them.  
Call out any **user feedback** that led you to change your solution or debugging approach.

### 5. **Problem Solving**  
Summarize all **problems solved**, including any **ongoing troubleshooting** efforts.

### 6. **All User Messages**  
Include **all user messages** (excluding tool output).  
These are essential for tracking **user feedback** and **shifts in intent**.

### 7. **Pending Tasks**  
List any **outstanding tasks** that the user explicitly asked you to work on.

### 8. **Current Work**  
Describe **precisely** what was being worked on **immediately before** the summary request.  
Include:
- File names
- Code snippets
- Specifics from the latest conversation  
Make sure this ties directly to the user’s **latest instructions**.

### 9. **Optional Next Step**  
Only include this if:
- It is a **direct continuation** of your last task
- It clearly aligns with the user’s **explicit request**

> **⚠️ Do not introduce new directions** unless confirmed with the user.

If appropriate, include **verbatim quotes** from the recent conversation to show **where you left off**.
