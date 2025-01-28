---
title: Quick Tips
---

Here is a collection of tips for working with Goose:

### Goose can and will edit files
Use a git strategy to avoid losing anything - such as staging your personal edits and leaving Goose edits unstaged until reviewed. Consider using individual commits which can be reverted. See more on [File Management](/docs/guides/file-management).

### Goose can and will run commands
Goose will check commands for satefy, however, you can also ask Goose to check with you before running commands if you are concerned.

### You can interrupt Goose with `CTRL+C`
Use this command to stop Goose in the event that you need to correct it or give it more info.

### Goose works best when solving concrete problems
Experiment with how far you need to break a problem down to get Goose to solve it. Be specific! For example, it will likely fail to "create a banking app", but probably does a good job if prompted with "create a Fastapi app with an endpoint for deposit and withdrawal and with account balances stored in mysql keyed by id".

### Goose can troubleshoot
If something goes wrong, Goose can help troubleshoot issues by examining logs, analyzing error messages, and suggesting possible resolutions.

### Leverage Goose to learn
Use Goose to learn new technologies or frameworks by asking it to explain things like code snippets, concepts, or best practices relevant to your project.

### Goose needs context
If Goose doesn't have enough context to start with, it might go in the wrong direction. Tell it to read files that you are referring to or search for objects in code. Even better, ask it to summarize them for you, which will help it set up its own next steps. You can create a [goosehints](/docs/guides/using-goosehints) file to help with this.

### Use easy search terms
Refer to any objects in files with something that is easy for Goose to search for, such as "the MyExample class".

### Teach Goose how you test
Goose *loves* to know how to run tests to get a feedback loop going, just like you do. If you tell it how you test things locally and quickly, it can make use of that when working on your project.

### Goose can do scripting tasks
You can use Goose for tasks that would require scripting. It can even look at your screen and correct designs, or help you fix bugs. Try asking it to help you in a way you would ask a person.

### Goose will make mistakes
Sometimes Goose will go in the wrong direction. Feel free to correct it, or start over again.

### Goose can run tasks continuously if asked
You can tell Goose to run things for you continuously and it will iterate, try, and retry.

### Goose can run anywhere
It doesn't have to be in a repo. Just ask Goose!

### Keep Goose updated
Regularly update Goose to benefit from the latest features, bug fixes, and performance improvements. For the CLI, the best way to keep it updated is by re-running the [Goose installation script][installation]. For Goose Desktop, check the [GitHub Releases page][ui-release] regularly for updates.

[installation]: https://block.github.io/goose/docs/quickstart/#installation
[ui-release]: https://github.com/block/goose/releases/stable