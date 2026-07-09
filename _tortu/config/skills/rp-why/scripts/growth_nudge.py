"""
Growth Nudge - Three Dimensions Assessment & Actionable Feedback

Provides practitioners with contextual feedback based on their
diagnostic zone in the TM x DOK matrix.

Thin wrapper around rp_why_core for quick assessments.
"""

from __future__ import annotations

from typing import List, Tuple

from rp_why_core import (
    DOK_NAMES,
    DOK_PATTERNS,
    TM_TIERS,
    ZONE_NUDGES,
    ZONE_REFLECTIONS,
    calculate_adt_zone,
    classify_dok,
    get_zone_nudges,
    get_zone_reflection,
)


class GrowthNudge:
    """Contextual nudges based on diagnostic zone."""

    DOK_PATTERNS = DOK_PATTERNS
    TM_TIERS = TM_TIERS
    DOK_NAMES = DOK_NAMES
    ZONE_NUDGES = ZONE_NUDGES
    ZONE_REFLECTIONS = ZONE_REFLECTIONS

    def classify_dok(self, text: str) -> Tuple[int, float, List[str]]:
        """Classify text by DOK level."""
        return classify_dok(text)

    def calculate_zone(self, dok: float, tm_tier: int) -> str:
        """Calculate diagnostic zone from DOK x TM matrix."""
        return calculate_adt_zone(dok, tm_tier)

    def get_nudges(self, zone: str) -> List[str]:
        """Get nudges for a diagnostic zone."""
        return get_zone_nudges(zone)

    def get_reflection(self, zone: str) -> str:
        """Get reflection question for a diagnostic zone."""
        return get_zone_reflection(zone)

    def assess(self, task_description: str, tm_tier: int = 3) -> str:
        """
        Quick assessment of a task description.

        Args:
            task_description: What the user is working on
            tm_tier: Current Orchestra tier (1-6)

        Returns:
            Formatted assessment with zone, nudges, and reflection
        """
        dok_level, _confidence, keywords = self.classify_dok(task_description)
        zone = self.calculate_zone(float(dok_level), tm_tier)
        nudges = self.get_nudges(zone)
        reflection = self.get_reflection(zone)

        output = []
        output.append(f"DOK {dok_level} ({self.DOK_NAMES[dok_level]})")
        output.append(f"TM Tier {tm_tier} ({self.TM_TIERS[tm_tier]})")
        output.append(f"Diagnostic Zone: {zone}")
        output.append("")

        if keywords:
            output.append(f"Detected: {', '.join(keywords)}")
            output.append("")

        output.append("Nudges:")
        for i, nudge in enumerate(nudges[:2], 1):
            output.append(f"  {i}. {nudge}")
        output.append("")
        output.append(f"Reflection: {reflection}")

        return "\n".join(output)


if __name__ == '__main__':
    import sys

    nudger = GrowthNudge()

    if len(sys.argv) > 1:
        task = ' '.join(sys.argv[1:])
        print(nudger.assess(task, tm_tier=4))
    else:
        print("Usage: python growth_nudge.py <task description>")
        print()
        print("Example:")
        result = nudger.assess("Design a caching strategy with trade-offs for our API", tm_tier=4)
        print(result)
