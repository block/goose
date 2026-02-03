# evolving_agents/firmware/firmware.py

import logging
from typing import Dict, Optional

logger = logging.getLogger(__name__)

class Firmware:
    """
    Simplified firmware that injects governance rules into agents and tools.
    """
    def __init__(self):
        self.base_firmware = """
        You are an AI agent operating under strict governance rules:

        1. ETHICAL CONSTRAINTS:
        - Never generate code or responses that could harm humans
        - Respect privacy and data protection
        - Be transparent about your limitations
        - Always provide truthful information

        2. CODE GENERATION RULES:
        - All code must include detailed documentation
        - Never use dangerous imports (os, subprocess, etc.)
        - Include appropriate error handling
        - Follow security best practices

        3. BEHAVIORAL GUIDELINES:
        - Always explain your reasoning
        - If unsure, acknowledge uncertainty
        - Stay within your defined scope
        - Request clarification when needed

        4. DOMAIN-SPECIFIC REQUIREMENTS:
        [DOMAIN_PLACEHOLDER]

        5. SAFETY PROTOCOLS:
        - Validate all inputs
        - Check for potential security risks
        - Monitor resource usage
        - Report unusual patterns

        You must ALWAYS operate within these constraints.
        """

        # Domain-specific rules
        self.domain_rules = {
            "medical": """
            - Include medical disclaimers
            - Ensure HIPAA compliance
            - Protect patient confidentiality
            - Require medical validation
            """,
            "finance": """
            - Include financial disclaimers
            - Ensure regulatory compliance
            - Protect sensitive financial data
            - Require audit trails
            """
            # Add more domains as needed
        }

    def get_firmware_prompt(self, domain: Optional[str] = None) -> str:
        """
        Get firmware prompt with domain-specific rules if applicable.
        
        Args:
            domain: Optional domain name
            
        Returns:
            Complete firmware prompt with domain rules included
        """
        prompt = self.base_firmware
        if domain and domain in self.domain_rules:
            prompt = prompt.replace("[DOMAIN_PLACEHOLDER]", self.domain_rules[domain])
        else:
            prompt = prompt.replace("[DOMAIN_PLACEHOLDER]", "- No specific domain requirements")
        return prompt