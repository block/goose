"""Initializer of Auto Instrumentation of AG2 Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.ag2.ag2 import (
    conversable_agent,
    agent_run,
    agent_generate_reply,
    agent_receive,
    agent_send,
    groupchat_manager_run_chat,
    groupchat_select_speaker,
)
from openlit.instrumentation.ag2.async_ag2 import (
    async_conversable_agent,
    async_agent_run,
    async_agent_generate_reply,
    async_agent_receive,
    async_agent_send,
)

_instruments = ("ag2 >= 0.3.2",)


class AG2Instrumentor(BaseInstrumentor):
    """
    An instrumentor for AG2 client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("ag2")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")

        # sync conversable agent
        wrap_function_wrapper(
            "autogen.agentchat.conversable_agent",
            "ConversableAgent.__init__",
            conversable_agent(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # sync agent run
        wrap_function_wrapper(
            "autogen.agentchat.conversable_agent",
            "ConversableAgent.run",
            agent_run(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # sync agent generate_reply
        wrap_function_wrapper(
            "autogen.agentchat.conversable_agent",
            "ConversableAgent.generate_reply",
            agent_generate_reply(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # sync agent receive
        wrap_function_wrapper(
            "autogen.agentchat.conversable_agent",
            "ConversableAgent.receive",
            agent_receive(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # sync agent send
        wrap_function_wrapper(
            "autogen.agentchat.conversable_agent",
            "ConversableAgent.send",
            agent_send(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # sync groupchat manager run_chat
        wrap_function_wrapper(
            "autogen.agentchat.groupchat",
            "GroupChatManager.run_chat",
            groupchat_manager_run_chat(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # sync groupchat select_speaker
        wrap_function_wrapper(
            "autogen.agentchat.groupchat",
            "GroupChat.select_speaker",
            groupchat_select_speaker(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

    def _uninstrument(self, **kwargs):
        # Proper uninstrumentation logic to revert patched methods
        pass
