"""
rp-why Core: Shared constants, classifiers, and zone computation.

This module is the single source of truth for:
- DOK classification patterns and logic
- Orchestra Tier (TM) definitions
- ADT diagnostic zone computation
- Compression detection
- System message filtering
- Growth nudges by zone

Imported by: rp_why_baseline.py, growth_nudge.py, goose_skill.py
"""

from __future__ import annotations

import re
from typing import List, Tuple


# ═══════════════════════════════════════════════════════════════════
# CONSTANTS
# ═══════════════════════════════════════════════════════════════════

DOK_PATTERNS: dict = {
    1: [
        # Information retrieval, recall, simple lookups
        r'\bhow do i\b', r'\bwhat is\b', r'\bsyntax\b', r'\bcommand\b',
        r'\bwhere\b', r'\bshow me\b', r'\bexample\b', r'\bhelp with\b',
        r'\bwhat\'s the\b', r'\bhow to\b', r'\bcan you show\b',
        r'\blist\b', r'\bdefine\b', r'\blook up\b',
        r'\blook at\b', r'\btake a look\b', r'\bfind the\b',
        r'\bread the\b', r'\bopen the\b', r'\bget the\b',
        r'\bcheck on\b', r'\bdisplay\b', r'\bpull up\b',
        r'\bwhat does .+ (do|mean|say)\b',
        r'\bremind me\b', r'\bwhere is\b', r'\bwhere are\b',
    ],
    2: [
        # Applying known patterns, executing within familiar contexts
        r'\bimplement\b', r'\bdebug\b', r'\bfix\b', r'\brefactor\b',
        r'\btest\b', r'\badd\b', r'\bupdate\b', r'\bcreate\b',
        r'\bbuild\b', r'\bwrite\b', r'\bmodify\b', r'\bchange\b',
        r'\bremove\b', r'\bdelete\b', r'\binstall\b', r'\bsetup\b',
        r'\bconfigure\b', r'\bmigrate\b', r'\bconvert\b',
        r'\brun\b', r'\bexecute\b', r'\bdeploy\b', r'\bformat\b',
        r'\brename\b', r'\bmove\b', r'\bcopy\b', r'\bmerge\b',
        r'\bcommit\b', r'\bpush\b', r'\brebase\b',
    ],
    3: [
        # Strategic reasoning, trade-offs, planning, evaluation
        r'\bdesign\b', r'\barchitect\b', r'\banalyze\b', r'\bcompare\b',
        r'\btrade-?off\b', r'\bbest approach\b', r'\bevaluate\b',
        r'\bstrategy\b', r'(?<![-\w])why\b', r'\bimplications\b', r'\bshould we\b',
        r'\breview\b', r'\boptimize\b', r'\bimprove\b', r'\balternative\b',
        r'\bpros and cons\b', r'\bdecision\b', r'\bplan\b',
        r'\bwhat if\b', r'\bconsequences\b', r'\bprioritize\b',
        r'\bvalidat\w*\b', r'\bverif\w*\b', r'\bensur\w*\b', r'\bsound\b',
        r'\binteract\w*\b', r'\bassumption\w*\b', r'\bwhat would break\b',
        r'\bedge case\w*\b', r'\bhold\w* up\b',
        r'\bthink through\b', r'\breason\w* about\b', r'\bweigh\b',
        r'\bconsider\b', r'\bdepend\w* on\b',
        # Evaluative/quality/alignment language
        r'\bmake sure\b', r'\bensure that\b',
        r'\bshould be\b', r'\bsupposed to\b',
        r'\binstead of\b', r'\brather than\b',
        r'\bappropriate\b', r'\bcorrect\w*\b', r'\balign\w*\b',
        r'\bdistinction\b', r'\bdifference between\b',
        r'\brelat\w+ to\b', r'\binterplay\b',
        r'\bobserv\w*\b', r'\bglean\b', r'\bsignifican\w*\b',
        r'\bmethodology\b', r'\bapproach\b',
        r'\bhow does .+ (relate|connect|fit|work with)\b',
        r'\bdiagnos\w*\b', r'\bassess\w*\b', r'\bmeasur\w*\b',
        r'\bcriteria\b', r'\brequirement\w*\b',
    ],
    4: [
        # Extended thinking, synthesis, creating novel frameworks
        r'\bresearch\b', r'\bnovel\b', r'\binnovate\b', r'\btransform\b',
        r'\bframework\b', r'\blong-term\b', r'\bevolve\b', r'\bvision\b',
        r'\bbreakthrough\b', r'\bsystematic\b', r'\bparadigm\b',
        r'\bfundamental\b', r'\bpioneer\b', r'\bgroundbreaking\b',
        r'\bsynthesize\b', r'\bcross-disciplinary\b', r'\boriginal\b',
        r'\bnew model\b', r'\btheory\b', r'\bhypothesis\b',
        r'\bpropos\w*\b', r'\bpublish\w*\b', r'\bcontribut\w*\b',
        r'\bextend\b', r'\bexpand upon\b', r'\bintegrat\w*\b',
        r'\bformulat\w*\b', r'\bconceptualiz\w*\b',
        r'\bcatalog\w*\b', r'\btaxonom\w*\b', r'\bclassif\w*\b',
    ]
}

TM_TIERS: dict = {
    1: "Solo",
    2: "Duet",
    3: "Ensemble",
    4: "Chamber",
    5: "Symphony",
    6: "Virtuoso",
}

DOK_NAMES: dict = {
    1: "Recall & Reproduction",
    2: "Application of Skills & Concepts",
    3: "Strategic Thinking",
    4: "Extended Thinking",
}

DOK_NAMES_SHORT: dict = {
    1: "Recall",
    2: "Application",
    3: "Strategic",
    4: "Extended",
}

ADT_ZONES: list = [
    "Overpowered", "Underutilizing", "Expected",
    "Growing", "Frontier", "Thinking Ahead",
]

ZONE_COLORS: dict = {
    "Expected": "grey",
    "Growing": "blue",
    "Frontier": "green",
    "Thinking Ahead": "purple",
    "Underutilizing": "amber",
    "Overpowered": "red",
}

SYSTEM_MESSAGE_PATTERNS: list = [
    r'^A (Python|shell|bash|TypeScript) (script|command|execution) was (executed|attempted|run|performed)',
    r'^Retrieved (the|a|lines) .+ (file|document|contents|data|from|interface|class|method)',
    r'^The (assistant|user|developer|tool|search|grep|build|shell) (retrieved|executed|ran|displayed|checked|searched|found|confirmed|examined|updated|added|created|removed)',
    r'^A (file edit|directory tree|shell command|delegation request|background task|code edit|git|grep|search|build|new|import)',
    r'^(File|Directory) (was|tree|listing)',
    r'^A cached (HTML|file|document)',
    r'^A (git|grep|code|recursive|file|Gradle|TypeScript|GitHub|Linear) .+ was (executed|performed|made|run|attempted|retrieved|created|updated)',
    r'^(An|The) (import|code|method|function|dependency|constructor|test|build|emulator|Android|app)',
    r'^(Checked|Verified|Examined|Updated|Added|Removed|Switched|Pushed|Committed)',
    r'^(Both|All|Two|Three|The existing|The new|The legacy|The Compose)',
    r'^(Good|Done|Clean|Build) [-.] ',
    r'^A (new|Compose|Material|Kotlin|Java) .+ (was|file|class|interface)',
    r'^(Found|Confirmed|Resolved|Deployed|Installed|Launched|Untracked)',
    r'^A (dependency injection|DI|Dagger|Anvil) .+ (was|field|binding)',
    r'^(Re-review|CI|Unit test|Snapshot|Detekt|ktfmt)',
]

COMPRESSION_MAX_WORDS: int = 8
COMPRESSION_INDICATORS: list = [
    r'^proceed$', r'^do it$', r'^go$', r'^yes$', r'^continue$',
    r'^ship it$', r'^deploy$', r'^merge$', r'^lgtm$',
    r'^next$', r'^done$', r'^send$', r'^push$',
    r'^run it$', r'^build$', r'^start$', r'^finish$',
]

ZONE_NUDGES: dict = {
    'Frontier': [
        "Operating at the productive edge. Document what works for others.",
        "The collaboration is matched. Look for opportunities to extend into new domains.",
        "Strong session. Consider extending one thread into a multi-session investigation.",
    ],
    'Growing': [
        "Approaching a match. Keep pushing DOK 3+ work and the zone will shift.",
        "Consider: what's one workflow you could delegate more fully?",
        "Building momentum. Try framing one more task as a design decision rather than an execution request.",
    ],
    'Expected': [
        "Healthy starting position. Growth comes from asking 'why' before implementing.",
        "Try framing one task as a design decision rather than an execution request.",
        "Solid foundation. Ask 'what are the trade-offs?' before your next implementation prompt.",
    ],
    'Thinking Ahead': [
        "Cognitive depth exceeds tool sophistication. Time to adopt more powerful orchestration.",
        "Your thinking is ready for the next tier. Explore sub-agents or multi-step delegation.",
        "What tool or workflow would unlock the depth you are already thinking at?",
    ],
    'Underutilizing': [
        "Powerful tools deserve powerful questions. Before each prompt: can this be more strategic?",
        "Batch simple queries. Reserve the agent for work that requires reasoning.",
        "What is the most strategic question you could ask right now?",
    ],
    'Overpowered': [
        "Significant mismatch. Consider whether this task needs an autonomous agent.",
        "Opportunity: redirect this tool toward a problem that requires analysis or design.",
        "Is there a harder problem this tool should be pointed at?",
    ],
}

ZONE_REFLECTIONS: dict = {
    'Frontier': "What complex challenge could benefit from sustained exploration across your next few sessions?",
    'Growing': "What workflow could you delegate more fully to the agent?",
    'Expected': "What strategic question have you been avoiding?",
    'Thinking Ahead': "What tool or workflow would unlock the depth you are already thinking at?",
    'Underutilizing': "What is the most strategic question you could ask right now?",
    'Overpowered': "Is there a harder problem this tool should be pointed at?",
}


# ═══════════════════════════════════════════════════════════════════
# CLASSIFICATION FUNCTIONS
# ═══════════════════════════════════════════════════════════════════

def is_system_message(text: str) -> bool:
    """Detect system-generated messages (tool output summaries).

    These are NOT human prompts and should be excluded from DOK
    classification entirely.
    """
    if not text:
        return False
    for pattern in SYSTEM_MESSAGE_PATTERNS:
        if re.match(pattern, text, re.IGNORECASE):
            return True
    return False


def classify_dok(text: str, session_context_dok: float | None = None) -> Tuple[int, float, List[str]]:
    """
    Classify text by DOK level using multi-signal approach.

    Args:
        text: The prompt text to classify
        session_context_dok: Rolling average DOK for the current session.
            When provided and no keywords match, the classifier defaults
            to round(session_context_dok) instead of a fixed DOK 2.

    Returns:
        (dok_level, confidence, matched_keywords)
        DOK 0 indicates a system message (should be excluded from counts).
    """
    if not text:
        return (2, 0.3, [])

    if is_system_message(text):
        return (0, 0.0, ['[system_message]'])

    text_lower = text.lower()
    scores = {1: 0, 2: 0, 3: 0, 4: 0}
    matched = []

    for level, patterns in DOK_PATTERNS.items():
        for pattern in patterns:
            if re.search(pattern, text_lower):
                scores[level] += 1
                match = re.search(pattern, text_lower)
                if match:
                    matched.append(match.group().strip())

    total_matches = sum(scores.values())
    if total_matches == 0:
        if session_context_dok is not None:
            context_level = max(1, min(4, round(session_context_dok)))
            return (context_level, 0.2, ['[session_context]'])
        return (2, 0.3, [])

    max_score = max(scores.values())
    # Tie-break resolves to the LOWER band (conservative). The instrument
    # should not inflate scores on ambiguous evidence. A 1-1 split between
    # DOK 2 and DOK 3 keywords resolves to DOK 2.
    level = min(k for k, v in scores.items() if v == max_score)
    confidence = max_score / total_matches if total_matches > 0 else 0.3

    return (level, min(confidence, 1.0), matched[:5])


def detect_compression(text: str, session_prompt_index: int) -> bool:
    """Detect compressed intent (short prompts carrying complex meaning).

    A prompt is compressed when it's short (<=8 words), appears after
    context is established (index > 2), and either matches a known
    compression pattern or is very short (<=4 words) deep in a session.
    """
    if not text:
        return False

    word_count = len(text.split())

    if word_count <= COMPRESSION_MAX_WORDS and session_prompt_index > 2:
        text_lower = text.lower().strip()
        for pattern in COMPRESSION_INDICATORS:
            if re.match(pattern, text_lower):
                return True
        # Short directive-style prompts deep in a session, but only if
        # they look imperative (2-4 words, not single characters/typos)
        if 2 <= word_count <= 4 and session_prompt_index > 5:
            return True

    return False


def calculate_adt_zone(dok_adjusted: float, tm_tier: int,
                       trajectory: str | None = None) -> str:
    """
    Calculate diagnostic zone from DOK x TM matrix.

    Trajectory-aware: when trajectory is 'improving' (DOK trending
    upward over the measurement window), the zone is upgraded one
    level toward Frontier. TM is stable at the baseline level for
    most practitioners, so DOK growth is the primary trajectory
    signal. The upgrade reflects that sustained cognitive growth
    indicates a healthier collaboration relationship than the
    static position alone would suggest.

    Zone hierarchy (low -> high):
      Overpowered -> Underutilizing -> Expected -> Growing -> Frontier
    Orthogonal zone (high DOK, low TM):
      Thinking Ahead
    """
    if dok_adjusted >= 3.0:
        dok_band = 4
    elif dok_adjusted >= 2.5:
        dok_band = 3
    elif dok_adjusted >= 2.0:
        dok_band = 2
    else:
        dok_band = 1

    if tm_tier >= 5:
        if dok_band >= 3:
            zone = "Frontier"
        elif dok_band == 2:
            zone = "Underutilizing"
        else:
            zone = "Overpowered"
    elif tm_tier >= 3:
        if dok_band >= 4:
            zone = "Frontier"
        elif dok_band == 3:
            zone = "Growing"
        elif dok_band == 2:
            zone = "Expected"
        else:
            zone = "Overpowered"
    else:
        if dok_band >= 3:
            zone = "Thinking Ahead"
        elif dok_band == 2:
            zone = "Growing"
        else:
            zone = "Expected"

    if trajectory == 'improving' and zone not in ('Frontier', 'Thinking Ahead'):
        upgrade_map = {
            'Overpowered': 'Underutilizing',
            'Underutilizing': 'Growing',
            'Expected': 'Growing',
            'Growing': 'Frontier',
        }
        zone = upgrade_map.get(zone, zone)

    return zone


def estimate_tm_tier(session_data: dict) -> int:
    """Estimate Orchestra tier from session characteristics."""
    has_subagents = session_data.get('has_subagents', False)
    prompt_count = session_data.get('prompt_count', 0)
    unique_tools = session_data.get('unique_tools', 0)

    if has_subagents and prompt_count > 50:
        return 5  # Symphony
    elif has_subagents or (unique_tools > 5 and prompt_count > 20):
        return 4  # Chamber
    elif prompt_count > 10 and unique_tools > 3:
        return 3  # Ensemble
    elif prompt_count > 3:
        return 2  # Duet
    else:
        return 1  # Solo


def aggregate_session_metadata(session_meta: dict, session_ids: set) -> dict:
    """Combine metadata from multiple sessions into one aggregated dict.

    Used when analyzing all sessions from a single day. Produces a
    consistent combined_meta regardless of whether sessions have
    the newer 'tools_seen' field or only the legacy 'unique_tools' count.
    """
    all_tools: set = set()
    has_tools_seen = False

    for sid in session_ids:
        meta = session_meta.get(sid, {})
        if meta.get('tools_seen'):
            has_tools_seen = True
            all_tools.update(meta['tools_seen'])

    if not has_tools_seen:
        unique_tools_count = sum(
            session_meta.get(sid, {}).get('unique_tools', 0)
            for sid in session_ids
        )
    else:
        unique_tools_count = len(all_tools)

    return {
        'prompt_count': sum(
            session_meta.get(sid, {}).get('prompt_count', 0)
            for sid in session_ids
        ),
        'unique_tools': unique_tools_count,
        'has_subagents': any(
            session_meta.get(sid, {}).get('has_subagents', False)
            for sid in session_ids
        ),
        'accumulated_tokens': sum(
            session_meta.get(sid, {}).get('accumulated_tokens', 0)
            for sid in session_ids
        ),
        'accumulated_input': sum(
            session_meta.get(sid, {}).get('accumulated_input', 0)
            for sid in session_ids
        ),
        'accumulated_output': sum(
            session_meta.get(sid, {}).get('accumulated_output', 0)
            for sid in session_ids
        ),
    }


def get_zone_nudges(zone: str) -> List[str]:
    """Get actionable nudges for a diagnostic zone."""
    return ZONE_NUDGES.get(zone, ZONE_NUDGES['Expected'])


def get_zone_reflection(zone: str) -> str:
    """Get reflection question for a diagnostic zone."""
    return ZONE_REFLECTIONS.get(zone, "What could you explore more deeply?")
