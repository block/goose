# evolving_agents/adapters/openai_guardrails_adapter.py

import logging
from typing import Any, Callable, Dict, List, Optional, Union

from agents.guardrail import GuardrailFunctionOutput, InputGuardrail, OutputGuardrail, input_guardrail, output_guardrail
from agents.run_context import RunContextWrapper

from evolving_agents.firmware.firmware import Firmware

logger = logging.getLogger(__name__)

class OpenAIGuardrailsAdapter:
    """Adapter for converting Evolving Agents firmware to OpenAI Agents SDK guardrails"""
    
    @staticmethod
    @input_guardrail
    async def create_firmware_input_guardrail(
        context: RunContextWrapper[Any], 
        agent: Any, 
        input: Union[str, List[Any]]
    ) -> GuardrailFunctionOutput:
        """
        Default input guardrail based on firmware rules
        
        Args:
            context: The run context wrapper
            agent: The agent being guarded
            input: The input to check
            
        Returns:
            Guardrail function output
        """
        # Simple implementation - check if input contains dangerous keywords
        dangerous_keywords = ["rm -rf", "sudo", "exec(", "os.system(", "__import__", "eval("]
        
        input_text = input if isinstance(input, str) else str(input)
        
        for keyword in dangerous_keywords:
            if keyword in input_text.lower():
                return GuardrailFunctionOutput(
                    output_info={
                        "violation": "dangerous_keyword",
                        "keyword": keyword,
                        "source": "firmware"
                    },
                    tripwire_triggered=True
                )
        
        return GuardrailFunctionOutput(
            output_info={"result": "input passed firmware checks"},
            tripwire_triggered=False
        )
    
    @staticmethod
    @output_guardrail
    async def create_firmware_output_guardrail(
        context: RunContextWrapper[Any], 
        agent: Any, 
        output: Any
    ) -> GuardrailFunctionOutput:
        """
        Default output guardrail based on firmware rules
        
        Args:
            context: The run context wrapper
            agent: The agent being guarded
            output: The output to check
            
        Returns:
            Guardrail function output
        """
        # Check if output contains dangerous content
        dangerous_keywords = ["rm -rf", "sudo", "exec(", "os.system(", "__import__", "eval("]
        
        output_text = str(output)
        
        for keyword in dangerous_keywords:
            if keyword in output_text.lower():
                return GuardrailFunctionOutput(
                    output_info={
                        "violation": "dangerous_keyword_in_output",
                        "keyword": keyword,
                        "source": "firmware"
                    },
                    tripwire_triggered=True
                )
        
        return GuardrailFunctionOutput(
            output_info={"result": "output passed firmware checks"},
            tripwire_triggered=False
        )
    
    @staticmethod
    def create_input_guardrail_from_firmware(
        firmware: Firmware, 
        domain: Optional[str] = None
    ) -> InputGuardrail[Any]:
        """
        Create an OpenAI input guardrail from Evolving Agents firmware
        
        Args:
            firmware: The firmware instance
            domain: Optional domain for domain-specific rules
            
        Returns:
            OpenAI InputGuardrail
        """
        firmware_content = firmware.get_firmware_prompt(domain)
        
        @input_guardrail(name=f"firmware_{domain or 'general'}_guardrail")
        async def firmware_guardrail(
            context: RunContextWrapper[Any], 
            agent: Any, 
            input: Union[str, List[Any]]
        ) -> GuardrailFunctionOutput:
            # Check if input contains dangerous commands based on firmware rules
            dangerous_patterns = ["rm -rf", "sudo", "exec(", "os.system(", "__import__", "eval("]
            
            input_text = input if isinstance(input, str) else str(input)
            domain_specific_violations = []
            
            # Basic check - dangerous patterns
            for pattern in dangerous_patterns:
                if pattern in input_text.lower():
                    return GuardrailFunctionOutput(
                        output_info={
                            "violation": "dangerous_pattern",
                            "pattern": pattern,
                            "domain": domain or "general",
                            "firmware_rule": "Never use dangerous system commands or code execution"
                        },
                        tripwire_triggered=True
                    )
            
            # Domain-specific checks
            if domain == "finance":
                # Check for potential financial fraud indicators
                financial_fraud_indicators = ["transfer all funds", "wire money", "urgent payment", "authorize transaction"]
                for indicator in financial_fraud_indicators:
                    if indicator in input_text.lower():
                        domain_specific_violations.append({
                            "type": "finance_fraud_indicator",
                            "indicator": indicator
                        })
            
            elif domain == "medical":
                # Check for potential medical confidentiality issues
                pii_patterns = ["patient record", "ssn", "social security", "medical history"]
                for pattern in pii_patterns:
                    if pattern in input_text.lower():
                        domain_specific_violations.append({
                            "type": "medical_confidentiality",
                            "pattern": pattern
                        })
            
            # If domain-specific violations found, trigger the guardrail
            if domain_specific_violations:
                return GuardrailFunctionOutput(
                    output_info={
                        "violation": "domain_specific",
                        "domain": domain,
                        "details": domain_specific_violations
                    },
                    tripwire_triggered=True
                )
            
            # No violations found
            return GuardrailFunctionOutput(
                output_info={
                    "result": "passed",
                    "domain": domain or "general",
                    "message": "Input passes all firmware guardrails"
                },
                tripwire_triggered=False
            )
        
        return firmware_guardrail
    
    @staticmethod
    def create_output_guardrail_from_firmware(
        firmware: Firmware, 
        domain: Optional[str] = None
    ) -> OutputGuardrail[Any]:
        """
        Create an OpenAI output guardrail from Evolving Agents firmware
        
        Args:
            firmware: The firmware instance
            domain: Optional domain for domain-specific rules
            
        Returns:
            OpenAI OutputGuardrail
        """
        firmware_content = firmware.get_firmware_prompt(domain)
        
        @output_guardrail(name=f"firmware_{domain or 'general'}_output_guardrail")
        async def firmware_output_guardrail(
            context: RunContextWrapper[Any], 
            agent: Any, 
            output: Any
        ) -> GuardrailFunctionOutput:
            # Check if output contains problematic content based on firmware rules
            output_text = str(output)
            domain_specific_violations = []
            
            # Basic safety checks for all domains
            dangerous_patterns = ["rm -rf", "sudo", "exec(", "os.system(", "__import__", "eval("]
            for pattern in dangerous_patterns:
                if pattern in output_text.lower():
                    return GuardrailFunctionOutput(
                        output_info={
                            "violation": "dangerous_pattern",
                            "pattern": pattern,
                            "domain": domain or "general",
                            "firmware_rule": "Never output dangerous system commands or code execution"
                        },
                        tripwire_triggered=True
                    )
            
            # Domain-specific checks
            if domain == "finance":
                # Check for financial advice issues
                problematic_phrases = ["guaranteed investment", "risk-free", "sure thing", "will definitely"]
                for phrase in problematic_phrases:
                    if phrase in output_text.lower():
                        domain_specific_violations.append({
                            "type": "financial_advice_violation",
                            "phrase": phrase
                        })
            
            elif domain == "medical":
                # Check for definitive medical advice issues
                problematic_advice = ["you definitely have", "you must", "guaranteed to cure", "proven treatment"]
                for advice in problematic_advice:
                    if advice in output_text.lower():
                        domain_specific_violations.append({
                            "type": "medical_advice_violation",
                            "advice": advice
                        })
            
            # If domain-specific violations found, trigger the guardrail
            if domain_specific_violations:
                return GuardrailFunctionOutput(
                    output_info={
                        "violation": "domain_specific_output",
                        "domain": domain,
                        "details": domain_specific_violations
                    },
                    tripwire_triggered=True
                )
            
            # No violations found
            return GuardrailFunctionOutput(
                output_info={
                    "result": "passed",
                    "domain": domain or "general",
                    "message": "Output passes all firmware guardrails"
                },
                tripwire_triggered=False
            )
        
        return firmware_output_guardrail
    
    @staticmethod
    def convert_firmware_to_guardrails(
        firmware: Firmware, 
        domain: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Convert Evolving Agents firmware to OpenAI guardrails
        
        Args:
            firmware: The firmware instance
            domain: Optional domain for domain-specific rules
            
        Returns:
            Dictionary with input and output guardrails
        """
        return {
            "input_guardrail": OpenAIGuardrailsAdapter.create_input_guardrail_from_firmware(firmware, domain),
            "output_guardrail": OpenAIGuardrailsAdapter.create_output_guardrail_from_firmware(firmware, domain)
        }