import base64
import json

from elevenlabs import ElevenLabs as ElevenLabsClient
from elevenlabs import VoiceSettings
from mcp import types as mcp_types

from aci.common.logging_setup import get_logger
from aci.common.schemas.virtual_mcp import VirtualMCPAuthTokenData
from aci.virtual_mcp.executors.connectors.base import BaseConnector

logger = get_logger(__name__)


class ElevenLabs(BaseConnector):
    """Connector for ElevenLabs text-to-speech API."""

    def __init__(
        self,
        auth_token_data: VirtualMCPAuthTokenData,
    ):
        super().__init__(auth_token_data)
        self.client = ElevenLabsClient(api_key=auth_token_data.token)

    def create_speech(
        self,
        voice_id: str,
        text: str,
        model_id: str | None = "eleven_multilingual_v2",
        voice_settings: VoiceSettings | None = None,
        output_format: str = "mp3_44100_128",
    ) -> mcp_types.CallToolResult:
        """
        Converts text into speech using ElevenLabs API and returns base64-encoded audio.

        Args:
            voice_id: ID of the voice to be used
            text: The text that will be converted into speech
            model_id: Identifier of the model to use (defaults to eleven_multilingual_v2)
            voice_settings: Voice settings overriding stored settings for the given voice
            output_format: Output format of the generated audio. Formatted as codec_sample_rate_bit
            rate. Defaults to mp3_44100_128.

        Returns:
            mcp_types.CallToolResult: mcp compatible call tool result containing the response
            from the ElevenLabs API
        """
        logger.debug("Executing create_speech")

        # Use the ElevenLabs SDK to convert text to speech
        audio_generator = self.client.text_to_speech.convert(
            voice_id=voice_id,
            text=text,
            model_id=model_id,
            voice_settings=voice_settings,
            output_format=output_format,
        )

        # Convert the generator to bytes
        audio_bytes = b"".join(audio_generator)

        # Convert audio bytes to base64
        audio_base64 = base64.b64encode(audio_bytes).decode("utf-8")

        logger.debug(
            f"Generated speech, bytes={len(audio_bytes)}, voice_id={voice_id}, model_id={model_id}, output_format={output_format}"  # noqa: E501
        )

        result = {
            "audio_base64": audio_base64,
            "voice_id": voice_id,
            "text_length": len(text),
            "model_id": model_id,
            "output_format": output_format,
        }

        return mcp_types.CallToolResult(
            structuredContent=result,
            content=[mcp_types.TextContent(type="text", text=json.dumps(result))],
        )
