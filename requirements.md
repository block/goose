Mini project: add "SKILLS" extension to goose repo here. 

Guidelines: you are to do this only in rust, idiomatic as per the .goosehints, tested, do not add inane single line comments (minimal comments) or extra files, do not add documentation for it yet. Be smart, do not overengineer.
Leave just one skills.md guide doc in the root (which will document how it is implemented briefly and how it is used)

What skills are: 

Skills are found in either: 

~/.claude/skills/*skill name here*
or 
the working dir of a project .claude/skills

Not only .claude dir, but working dir/.goose may contain a skills directory, and also ~/.config/goose/skills may contain skills (you will see goose uses a standard way to find config dir). 


A skill is at its heard a SKILLS.md file in a directory: 

```markdown
---
name: your-skill-name
description: Brief description of what this Skill does and when to use it
---

# Your Skill Name

## Instructions
Provide clear, step-by-step guidance for Claude.

## Examples
Show concrete examples of using this Skill.
```

What is important is to parse the forematter of the yaml, for name, and description (that is the only structured data we really need from it)

supporting files:
mauy be additional files alongside SKILL.md in its dir:

my-skill/
├── SKILL.md (required)
├── reference.md (optional documentation)
├── examples.md (optional examples)
├── scripts/
│   └── helper.py (optional utility)
└── templates/
    └── template.txt (optional template)

Reference these files from SKILL.md (for example the following is a hypoethical skill file):


```markdown
For advanced usage, see [reference.md](reference.md).

Run the helper script:
```bash
python scripts/helper.py input.txt
```
Read these files only when needed, using progressive disclosure to manage context efficiently.


You can use gh cli to inspect skills here: https://github.com/anthropics/skills for more concrete examples (but we are implementing them in goose, we dont' need perfect compatibility with claudes choices)

Implementation requirements: 

* Implement this similar to the todo or chatrecall tools where they are implemented (chatrecall_extension.rs)

* it will have instructions which, based on where goose is running, looks in the workingDir for .claude/skills or .goose/skills for directories wehich have said skills, or ~/.config/goose/skills dir for the same. The instructions that the tool shows will say: 

```
You have these skills at your disposal, when it is clear they can help you solve a problem or you are asked to use them: 
Skill name (name field of the yaml snippet): descscription of skill from the forematter of .md (in the description field of yaml snippet)
```

So the instructions are a list of skills

* *there is one tool which is "loadSkill" - it will take the name of the skill (from above) and then return the rest of the body of the skill. It will also in its response, mention the full path to where the skill dir is, with supporting files (such as script files, template files and other peer files alongside the skills.md - it will say `use the view file tools to access these files as needed, or run scripts as directed with dev extension`)

* This is a built in extension similar to the other ones

* Should be simply and meaningfully unit tested

* avoid making changes outside of the new extension, it should be a fairly additive change and not need anything or a lot that is cross cutting to make it work (just the same as what todo and chatrecall need really - similar to them)

End to end testing: 

when you think it should work, you can build or work out how to run the goose binary, you can run it from a tmp dir or your making with a .goose/skills dir with a skill in it and use `... goose --run -t 'please use skill x to y'` for whatever the skill does, to check that it uses that skill (vs other tools).
