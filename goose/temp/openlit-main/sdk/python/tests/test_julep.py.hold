# pylint: disable=duplicate-code, no-name-in-module, invalid-name, global-statement
"""
This module contains tests for Julep functionality using the Julep Python library.

Tests cover various SDK functions. 
These tests validate integration with OpenLIT.

Environment Variables:
    - JULEP_API_KEY: OpenAI API key for authentication.

Note: Ensure the environment is properly configured for Julep and OpenLIT monitoring
prior to running these tests.
"""

from julep import Julep
import yaml
import openlit

# Initialize synchronous Julep client
sync_client = Julep()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-python-testing", application_name="openlit-python-julep-test")

agent_id = None
task_id = None

TASK_YAML = """
name: Storyteller
description: Create a story based on an idea.

tools:
  - name: research_wikipedia
    type: integration
    integration:
      provider: wikipedia
      method: search

main:
  # Step 1: Generate plot idea
  - prompt:
      - role: system
        content: You are {{agent.name}}. {{agent.about}}
      - role: user
        content: >
          Based on the idea '{{_.idea}}', generate a list of 5 plot ideas.
          Go crazy and be as creative as possible.
          Return your output as a list of long strings inside ```yaml 
          tags at the end of your response.
    unwrap: true

  - evaluate:
      plot_ideas: load_yaml(_.split('```yaml')[1].split('```')[0].strip())

  # Step 2: Extract research fields from the plot ideas
  - prompt:
      - role: system
        content: You are {{agent.name}}. {{agent.about}}
      - role: user
        content: >
          Here are some plot ideas for a story:
          {% for idea in _.plot_ideas %}
          - {{idea}}
          {% endfor %}

          To develop the story, we need to research for the plot ideas.
          What should we research? Write down wikipedia search queries for the plot ideas you think are interesting.
          Return your output as a yaml list inside ```yaml tags at the end of your response.
    unwrap: true
    settings:
      model: gpt-4o-mini
      temperature: 0.7

  - evaluate:
      research_queries: load_yaml(_.split('```yaml')[1].split('```')[0].strip())

  # Step 3: Research each plot idea
  - foreach:
      in: _.research_queries
      do:
        tool: research_wikipedia
        arguments:
          query: _

  - evaluate:
      wikipedia_results: 'NEWLINE.join([f"- {doc.metadata.title}: {doc.metadata.summary}" for item in _ for doc in item.documents])'

  # Step 4: Think and deliberate
  - prompt:
      - role: system
        content: You are {{agent.name}}. {{agent.about}}
      - role: user
        content: |-
          Before we write the story, let's think and deliberate. Here are some plot ideas:
          {% for idea in outputs[1].plot_ideas %}
          - {{idea}}
          {% endfor %}

          Here are the results from researching the plot ideas on Wikipedia:
          {{_.wikipedia_results}}

          Think about the plot ideas critically. Combine the plot ideas with the results from Wikipedia to create a detailed plot for a story.
          Write down all your notes and thoughts.
          Then finally write the plot as a yaml object inside ```yaml tags at the end of your response. The yaml object should have the following structure:

          ```yaml
          title: "<string>"
          characters:
          - name: "<string>"
            about: "<string>"
          synopsis: "<string>"
          scenes:
          - title: "<string>"
            description: "<string>"
            characters:
            - name: "<string>"
              role: "<string>"
            plotlines:
            - "<string>"```

          Make sure the yaml is valid and the characters and scenes are not empty. Also take care of semicolons and other gotchas of writing yaml.
    unwrap: true

  - evaluate:
      plot: "load_yaml(_.split('```yaml')[1].split('```')[0].strip())"
"""

def test_sync_create_agent():
    """
    Tests synchronous creation of agent

    Raises:
        AssertionError: If the agent creation response object is not as expected.
    """

    global agent_id

    try:
        response = sync_client.agents.create(
            name="Observability Expert",
            about="You are a AI Observability Expert.",
        )
        agent_id = response.id
        assert isinstance(agent_id, str)

    # pylint: disable=broad-exception-caught, try-except-raise
    except Exception:
        raise

def test_sync_task_create():
    """
    Tests synchronous creation of task

    Raises:
        AssertionError: If the task creation response object is not as expected.
    """

    global task_id

    try:
        task = sync_client.tasks.create(
            agent_id=agent_id,
            **yaml.safe_load(TASK_YAML)
        )
        task_id = task.id
        assert isinstance(task_id, str)

    # pylint: disable=broad-exception-caught, try-except-raise
    except Exception:
        raise

def test_sync_create_execution():
    """
    Tests synchronous creation of execution

    Raises:
        AssertionError: If the execution creation response object is not as expected.
    """

    try:
        execution = sync_client.executions.create(
            task_id=task_id,
            input={"idea": "A cat who learns to fly"}
        )
        assert isinstance(execution.id, str)

    # pylint: disable=broad-exception-caught, try-except-raise
    except Exception:
        raise
