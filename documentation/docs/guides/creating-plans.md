---
sidebar_position: 17
title: Creating Plans with the /plan prompt
sidebar_label: Creating Plans
---

Starting a project without a clear plan is like building a house without a blueprint. It can lead to:

* Confusion about what to do
* Wasted time and effort
* Projects that grow too big

A good plan keeps everyone on track and helps measure progress. That's why the Goose CLI includes the `/plan` prompt completion command to help break down your projects into clear, manageable steps.
 
:::tip Plans in the Goose desktop
The Goose desktop doesn't have a `plan` keyword. If you want the desktop to create a plan for you, you need to use a prompt like:

```
"Hey Goose, can you create a plan to convert my CLI project into a locally hosted web page that gives me input fields for each CLI command I can run? Please don't start the actual work"
```
Unless you ask Goose to "create a plan", it might just start into the project work. 
:::


## Set your planner provider and model
The Goose CLI plan mode uses two configuration values:

- `GOOSE_PLANNER_PROVIDER`: Which provider to use for planning
- `GOOSE_PLANNER_MODEL`: Which model to use for planning

For example, you might configure:
```bash
export GOOSE_PLANNER_PROVIDER=databricks
export GOOSE_PLANNER_MODEL=goose-claude-3-7-sonnet
```
If these aren't set, Goose will use your default provider and model settings. You might want to set different planning models if you find certain models are better at breaking down tasks into clear steps. However, your default model configuration is usually sufficient.

To verify that the planner provider is set input the following terminal command:

```bash
~ goose info -v
```

In this example, the `info` command returns the current configuration and the path to the configuration file.  

```bash
Goose Version:
  Version:          1.0.18

Goose Locations:
  Config file:      /Users/jaustin/.config/goose/config.yaml
  Sessions dir:     /Users/jaustin/.local/share/goose/sessions
  Logs dir:         /Users/jaustin/.local/state/goose/logs

Goose Configuration:
  GOOSE_PROVIDER: databricks
  DATABRICKS_HOST: https://block-lakehouse-production.cloud.databricks.com/
  GOOSE_MODEL: databricks-meta-llama-3-3-70b-instruct
  GOOSE_PLANNER_PROVIDER: databricks
  GOOSE_MODE: smart_approve
  GOOSE_PLANNER_MODEL: goose-claude-3-7-sonnet
```

 If either `GOOSE_PLANNER_PROVIDER` or `GOOSE_PLANNER_MODEL` are not set, `GOOSE_PROVIDER` and `GOOSE_MODEL`are used to build your plan.  

## Describe your project
The Goose CLI can handle project descriptions of almost any complexity. However, it's best if you make your project description succinct and to the point. Goose needs to understand the purpose of your project and ideal outcomes before it can create a project plan. If your project description is vague then Goose is going to ask clarifying questions until it believes it understands your project. 

If Goose believes your project can be completed in many different ways and using a wide variety of components, it will ask you a clarifying question for each of these decision points. For example, if you start a plan like this:

```bash 
Goose is running! Enter your instructions, or try asking what goose can do.

( O)> /plan

Entering plan mode. You can provide instructions to create a plan and then act on it. To exit early, type /endplan

( O)> Convert the CLI at /Users/jaustin/Development/devs-content/contentful-goose-script/search_replace_routes.py into a web page
```

## Clarifying questions
Converting a Python CLI into a web site seems simple enough but Goose will have questions about things like styling, authentication, features, technology stack, and more. You might see questions like this:

```bash
1. Should the application support any keyboard shortcuts for common actions?
2. Would you like the application to remember user preferences (like case sensitivity setting) between sessions?
3. Should there be any form of notification when operations complete successfully?
4. How should the application handle very large text fields that might be difficult to display in the three-column layout?
5. Are there any specific CI/CD requirements for deployment to AWS?
6. Will you need any automated testing implemented (unit tests, integration tests, etc.)?
7. Do you have any specific requirements for code organization or architecture patterns?
8. Should the application include any form of documentation or help section for users?
```
You can answer the questions one at a time or you can batch your answers:

```bash
( O)> 1 no keyboard shortcuts. 2 do not remember preferences. 3 on success, open a dialog that says "success". 4 Truncate to 30 characters before the string to replace and then 30 characters after the string to replace.5 no CI/CD requirements 6 No automated testing. 7. No specific requirement for organization or architecture. 8 no documentation
```


:::tip
Sometimes Goose will ask you to provide an artifact such as code from your project. In plan mode, Goose isn't going to be able to open the artifact file even if you give it the path. Instead, you need to paste the contents of the file at the Goose prompt. Prefix the pasted text with something like "Here's that source code you asked for"
:::

You should answer questions in a way that Goose understands which questions you are answering. In the previous example, each answer is preceeded by the question number. You can see that answers like "do not remember preferences" make more sense to Goose than simply "no" or "don't remember". If Goose doesn't connect your answer to its question, it may ask that question again.

For a project as complex as this example CLI to web site conversion project, Goose may ask sets of clarifying questions many times. Usually a new set of questions is generated because Goose learned more about your project from your previous answers. Sometimes your answers made Goose realize there's more to know about some aspect of your project... and then you get another Clarifying question.

If you've answered _all_ of Gooses questions and it has no more questions, Goose will generate the plan. Other times, you might think Goose will never run out of questions. If you want your plan and don't want to answer more questions, you can simply ask for a "generic" plan:

```bash
I still need some critical information to create a comprehensive plan:

1. What specific functionality does your Python script perform with Contentful? Could you share the actual code or describe how it interacts with the Contentful API to search and replace content?
2. What are the 5 predefined keys/inputs you mentioned earlier?
3. Should the web application include all the command-line options from the script (environment, status, case-sensitive, partial-match, dry-run)?
4. How should the application handle errors from the Contentful API?
5. Should the application maintain any history of previous search/replace operations?

Without this information, I can only provide a generic plan that might not accurately capture your requirements.
( O)> please provde a generic plan
```
Despite the fact that Goose produces a "generic" plan, it still takes into account the questions that you _have_ answered.

## A simple construction plan example
Goose can produce good plans for relatively simple projects such as the home construction example:

```bash

( O)> /plan

Entering plan mode. You can provide instructions to create a plan and then act on it. To exit early, type /endplan

( O)> Build a four bedroom house
◓  Measuring semantic distance...                                                                                                         

I need to gather more information before creating a detailed house-building plan. Please provide clarification on the following:

1. What is the purpose of this plan? Are you looking for architectural designs, construction steps, or a project management timeline?
2. What is your budget range for this four-bedroom house?
3. What is the approximate square footage or size requirements?
4. Do you have a specific location or lot already selected? What are the dimensions and any zoning restrictions?
5. Are there any specific architectural styles or features you want included?
6. How many bathrooms and other rooms (e.g., kitchen, living room, dining room, garage) do you want?
7. Do you have any specific sustainability or energy efficiency requirements?
8. What is your timeline for completion?
9. Are you working with any professionals already (architects, contractors, etc.)?
10. Are there any special considerations for the house (accessibility features, home office space, etc.)?
( O)> 1 construction steps and a project management timeline. 2 $600,000. 3 4,000 sq feet. 4 the lot is not yet selected. 5 one story rambler. 6 four bedrooms, two ful baths, a kitchen, diningroom, family room, and garage. 7 the house must be efficient and powered by solar energy. 8 complete in six months. 9 I am working with an architect. 10 I need a home office
◓  Taming tensors...                                                                                                          I'll create a detailed plan for building a 4,000 sq ft, one-story rambler with four bedrooms, two full bathrooms, kitchen, dining room, family room, garage, and home office. The house will be energy efficient with solar power, with a $600,000 budget and 6-month timeline.

◇  Do you want to clear message history & act on this plan?
│  Yes 
│

Exiting plan mode and acting on the above plan

### Introduction to Building a 4,000 Sq Ft Rambler
To create a detailed plan for your 4,000 sq ft, one-story rambler, we need to consider several factors including design, energy efficiency, budget, and timeline. Given your requirements, here's an overview of how to approach this project:

### Step 1: Design and Planning
- **Architectural Design**: Hire an architect to design the house, ensuring it meets your space requirements and is energy efficient. Consider factors like natural lighting, insulation, and window placement.
- **Floor Plan**: Create a floor plan that includes four bedrooms, two full bathrooms, a kitchen, dining room, family room, garage, and home office. Ensure the layout is functional and spacious.
- **Land Preparation**: Ensure the building site is prepared, including clearing the land, grading, and preparing the foundation.

### Step 2: Energy Efficiency and Solar Power
- **Solar Power System**: Design and install a solar power system that can meet the energy needs of a 4,000 sq ft house. This may include a large solar panel array and a battery storage system for nighttime and cloudy days.
- **Insulation and Windows**: Use high-quality insulation in the walls, floor, and ceiling to minimize heat loss. Install energy-efficient windows that allow natural light in while keeping the heat or cold out.
- **Energy-Efficient Appliances**: Specify energy-efficient appliances and lighting to minimize energy consumption.

### Step 3: Budgeting
- **Construction Costs**: Allocate approximately $200-250 per sq ft for construction, which translates to $800,000-$1,000,000 for a 4,000 sq ft house. However, your budget is $600,000, so you'll need to optimize costs without compromising on quality and energy efficiency.
- **Solar Power System Costs**: Budget an additional $20,000 to $50,000 for the solar power system, depending on its size and complexity.
- **Contingency Fund**: Set aside 10-20% of the budget for unexpected expenses.

### Step 4: Timeline
- **Month 1-2: Design and Planning**: Complete the architectural design, obtain necessary permits, and prepare the building site.
- **Month 3-4: Foundation and Framing**: Lay the foundation, frame the house, and install the roof.
- **Month 5: Installation of Electrical, Plumbing, and HVAC Systems**: Install the solar power system, electrical, plumbing, and HVAC systems.
- **Month 6: Finishing Work**: Complete the interior finishing work, including installation of energy-efficient appliances, flooring, and fixtures.

### Conclusion
Building a 4,000 sq ft, energy-efficient rambler with solar power within a $600,000 budget and 6-month timeline is challenging but feasible with careful planning and design. It's essential to work closely with your architect, contractor, and solar power system installer to ensure the project stays on track and within budget.

Given that no extensions are currently available for more specific tools or data sources, consider the following steps to proceed with your project:

* Search for available extensions using the <function=platform__search_available_extensions>{}</function>

```
The home construction plan has only high-level details because the model and planner Goose is using for the project does not have the necessary construction-specific knowledge. Currently Goose integrates with more technology and development-specific LLMs. This is why you'll get more clarifying questions and far more specific plan detail if your project resembles the first example.


## Basic usage
You need to have an active Goose session before you can put the CLI into plan mode. If you are going to dedicate a session to creating a plan, you should give your new session a name as in the following example:

```bash
~ goose session -n web-project-plan -r
resuming session | provider: databricks model: databricks-meta-llama-3-3-70b-instruct
    logging to /Users/jaustin/.local/share/goose/sessions/web-plan.jsonl
    working directory: /Users/jaustin

Goose is running! Enter your instructions, or try asking what goose can do.
```
To enter planning mode, type `/plan`.  Optionally, you can append your plan description to the prompt completion command.
```bash
( O)> /plan
```

 Plan mode in the CLI is a special interaction mode where Goose helps break down tasks into manageable steps.


