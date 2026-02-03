# evolving_agents/tools/internal/message_summarization_tool.py
import logging
from typing import List, Dict, Optional, Any, Type # Added Type

from pydantic import BaseModel, Field # For input/output schemas

from beeai_framework.tools.tool import Tool, StringToolOutput # Base Tool class
from beeai_framework.emitter.emitter import Emitter # Required by Tool
from beeai_framework.context import RunContext # Required for Tool._run signature

from evolving_agents.core.llm_service import LLMService

# Configure logging
logger = logging.getLogger(__name__)

class MessageSummarizationInput(BaseModel):
    """Input schema for the MessageSummarizationTool."""
    messages: List[Dict[str, Any]] = Field(..., description="A list of message dictionaries. Each dict should have 'sender' (or 'sender_id', 'user') and 'content' keys.")
    target_goal: str = Field(..., description="The goal to guide the summarization process, ensuring relevance.")
    max_summary_length_words: int = Field(
        default=150, 
        gt=10, 
        le=500,
        description="Approximate desired maximum length of the summary in words."
    )
    # Optional: Add other parameters if needed, like summarization style, language, etc.

class MessageSummarizationOutput(BaseModel): # Output schema for the tool
    status: str = Field(description="Status of the summarization ('success' or 'error').")
    summary_text: Optional[str] = Field(default=None, description="The generated summary text, if successful.")
    message: Optional[str] = Field(default=None, description="Details in case of an error or additional info.")
    input_message_count: int
    target_goal_for_summary: str

class MessageSummarizationTool(Tool[MessageSummarizationInput, None, MessageSummarizationOutput]):
    """
    A tool to summarize a list of messages using an LLM,
    focusing on aspects relevant to a specified target goal.
    This tool is typically used internally by the MemoryManagerAgent.
    """
    name: str = "MessageSummarizationTool"
    description: str = (
        "Summarizes a list of messages (e.g., conversation history) "
        "with a focus on a given target goal, using an LLM."
    )
    input_schema: Type[BaseModel] = MessageSummarizationInput
    output_schema: Type[BaseModel] = MessageSummarizationOutput

    def __init__(
        self, 
        llm_service: LLMService,
        options: Optional[Dict[str, Any]] = None # For Tool base class
    ):
        super().__init__(options=options) # Call Tool's __init__
        if llm_service is None:
            raise ValueError("llm_service cannot be None for MessageSummarizationTool")
        self.llm_service = llm_service

    def _create_emitter(self) -> Emitter: # Implement required method
        return Emitter.root().child(
            namespace=["tool", "internal", "message_summarization"], # Adjusted namespace
            creator=self,
        )

    async def _run(
        self, 
        input: MessageSummarizationInput, 
        options: Optional[Dict[str, Any]] = None, 
        context: Optional[RunContext] = None
    ) -> MessageSummarizationOutput:
        """
        Summarizes a list of messages focusing on a target goal.
        """
        if not input.messages:
            logger.warning(f"{self.name}: called with no messages.")
            return MessageSummarizationOutput(
                status="error", 
                message="No messages provided to summarize.",
                input_message_count=0,
                target_goal_for_summary=input.target_goal
            )
        if not input.target_goal:
            logger.warning(f"{self.name}: called with no target_goal.")
            return MessageSummarizationOutput(
                status="error", 
                message="No target goal provided for summarization.",
                input_message_count=len(input.messages),
                target_goal_for_summary=input.target_goal
            )

        formatted_messages = []
        for msg in input.messages:
            sender = msg.get("sender_id", msg.get("sender", msg.get("user", "UnknownSender")))
            content = msg.get("content", "")
            if content and isinstance(content, str): # Ensure content is a string
                formatted_messages.append(f"{sender}: {content}")
            elif content: # If content is not string but present, log a warning
                logger.debug(f"Message content from sender '{sender}' is not a string, converting: {type(content)}")
                formatted_messages.append(f"{sender}: {str(content)}")

        if not formatted_messages:
            logger.info(f"{self.name}: No messages with processable content found to summarize.")
            return MessageSummarizationOutput(
                status="success", # Technically successful as there's nothing to summarize
                summary_text="No message content available to summarize.",
                input_message_count=len(input.messages),
                target_goal_for_summary=input.target_goal
            )

        messages_string = "\n".join(formatted_messages)

        # Construct the prompt for the LLM
        prompt = f"""Please review the following conversation history:
--- BEGIN CONVERSATION ---
{messages_string}
--- END CONVERSATION ---

The primary goal for understanding this conversation is: "{input.target_goal}"

Your task is to provide a concise summary of the conversation. The summary MUST focus on information directly relevant to achieving or understanding progress towards the stated goal.
Aim for a summary of approximately {input.max_summary_length_words} words.
Extract key points, decisions, questions, or context that would help someone quickly grasp the conversation's relevance to the goal.

Concise Summary:"""

        try:
            logger.debug(f"{self.name}: Sending summarization prompt to LLM (goal: {input.target_goal}, {len(formatted_messages)} messages).")
            # Assuming llm_service.generate() is an async method
            summary_text = await self.llm_service.generate(prompt)
            
            if not summary_text or summary_text.strip() == "":
                logger.warning(f"{self.name}: LLM returned an empty summary for goal: {input.target_goal}")
                return MessageSummarizationOutput(
                    status="success", # Or "warning" if you prefer
                    summary_text="Summary generation resulted in an empty response.",
                    message="LLM returned an empty or blank summary.",
                    input_message_count=len(input.messages),
                    target_goal_for_summary=input.target_goal
                )
            
            return MessageSummarizationOutput(
                status="success",
                summary_text=summary_text.strip(),
                input_message_count=len(input.messages),
                target_goal_for_summary=input.target_goal
            )
            
        except Exception as e:
            logger.error(f"{self.name}: Error during LLM call for message summarization: {e}", exc_info=True)
            return MessageSummarizationOutput(
                status="error",
                message=f"Error summarizing messages: {str(e)}",
                input_message_count=len(input.messages),
                target_goal_for_summary=input.target_goal
            )

# Example Usage (Conceptual - requires full EAT setup)
# async def main_example():
#     # ... (Setup container with LLMService) ...
#     # llm_service = container.get('llm_service')
#     # summarization_tool = MessageSummarizationTool(llm_service=llm_service)

#     # sample_messages_input = MessageSummarizationInput(
#     #     messages=[
#     #         {"sender_id": "Alice", "content": "Hey Bob, are we still on for dinner tonight? I was thinking Italian."},
#     #         {"sender_id": "Bob", "content": "Yes! Italian sounds great. Any specific place downtown?"},
#     #         {"sender_id": "Alice", "content": "Not sure yet, let's find a place with good pasta reviews."},
#     #         {"sender_id": "System", "content": {"action_taken": "search_restaurants", "parameters": {"cuisine": "Italian", "area": "downtown"}}},
#     #         {"sender_id": "System", "content": "Found 'Luigi's Pasta Place' with 4.5 stars."},
#     #         {"sender_id": "Alice", "content": "Perfect, let's go there!"}
#     #     ],
#     #     target_goal="Finalize dinner plans and select a restaurant.",
#     #     max_summary_length_words=75
#     # )

#     # print("\n--- Testing MessageSummarizationTool ---")
#     # result = await summarization_tool._run(input=sample_messages_input)
#     # print(f"Summarization Tool Output:\n{result.model_dump_json(indent=2)}")
#     pass

# if __name__ == "__main__":
#     import asyncio
#     # asyncio.run(main_example())
#     pass