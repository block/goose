#!/usr/bin/env python3
"""
RP-WHY Baseline Analysis Tool (v4 - Three Dimensions)

Analyzes historical Goose sessions to establish a DOK/TM/ADT baseline.
Uses all available session data.

Usage:
    python rp_why_baseline.py init      # Generate baseline analysis
    python rp_why_baseline.py baseline  # Same as init
    python rp_why_baseline.py current   # Analyze current/recent session
    python rp_why_baseline.py compare   # Compare to baseline
    python rp_why_baseline.py overall   # Full longitudinal report
    python rp_why_baseline.py export    # Export baseline as JSON
    python rp_why_baseline.py show      # Show current baseline
"""

import sqlite3
import json
import os
from datetime import datetime
from pathlib import Path
from typing import ClassVar, Dict, List, Tuple, Optional
from collections import defaultdict

from rp_why_core import (
    DOK_PATTERNS as CORE_DOK_PATTERNS,
    TM_TIERS as CORE_TM_TIERS,
    DOK_NAMES as CORE_DOK_NAMES,
    DOK_NAMES_SHORT as CORE_DOK_NAMES_SHORT,
    ADT_ZONES as CORE_ADT_ZONES,
    SYSTEM_MESSAGE_PATTERNS as CORE_SYSTEM_MESSAGE_PATTERNS,
    COMPRESSION_MAX_WORDS as CORE_COMPRESSION_MAX_WORDS,
    COMPRESSION_INDICATORS as CORE_COMPRESSION_INDICATORS,
    classify_dok as core_classify_dok,
    is_system_message as core_is_system_message,
    detect_compression as core_detect_compression,
    calculate_adt_zone as core_calculate_adt_zone,
    estimate_tm_tier as core_estimate_tm_tier,
    aggregate_session_metadata,
)


class RPWhyAnalyzer:
    """Analyze Goose sessions using the Three Dimensions model"""

    # Database and config paths (cross-platform)
    @staticmethod
    def _get_sessions_db() -> Path:
        import platform
        if platform.system() == "Windows":
            return Path(os.environ.get("LOCALAPPDATA", "")) / "goose" / "sessions" / "sessions.db"
        else:
            return Path.home() / ".local/share/goose/sessions/sessions.db"

    @staticmethod
    def _get_baseline_file() -> Path:
        import platform
        if platform.system() == "Windows":
            return Path(os.environ.get("LOCALAPPDATA", "")) / "goose" / "rp-why-baseline.json"
        else:
            return Path.home() / ".config/goose/rp-why-baseline.json"

    # All constants imported from rp_why_core (single source of truth)
    DOK_PATTERNS: ClassVar[dict] = CORE_DOK_PATTERNS
    SYSTEM_MESSAGE_PATTERNS: ClassVar[list] = CORE_SYSTEM_MESSAGE_PATTERNS
    COMPRESSION_MAX_WORDS = CORE_COMPRESSION_MAX_WORDS
    COMPRESSION_INDICATORS: ClassVar[list] = CORE_COMPRESSION_INDICATORS
    TM_TIERS: ClassVar[dict] = CORE_TM_TIERS
    ADT_ZONES: ClassVar[list] = CORE_ADT_ZONES
    DOK_NAMES: ClassVar[dict] = CORE_DOK_NAMES
    DOK_NAMES_SHORT: ClassVar[dict] = CORE_DOK_NAMES_SHORT

    def __init__(self):
        self.conn = None

    def connect(self) -> bool:
        if not self._get_sessions_db().exists():
            print(f"Sessions database not found at {self._get_sessions_db()}")
            return False
        try:
            self.conn = sqlite3.connect(str(self._get_sessions_db()))
            self.conn.row_factory = sqlite3.Row
            return True
        except sqlite3.Error as e:
            print(f"Database connection error: {e}")
            return False

    def close(self):
        if self.conn:
            self.conn.close()

    # --- Classification (delegates to rp_why_core) -------------------------

    def is_system_message(self, text: str) -> bool:
        """Detect system-generated messages (tool output summaries)."""
        return core_is_system_message(text)

    def classify_dok(self, text: str, session_context_dok: float | None = None) -> Tuple[int, float, List[str]]:
        """Classify text by DOK level using multi-signal approach."""
        return core_classify_dok(text, session_context_dok)

    def detect_compression(self, text: str, session_prompt_index: int) -> bool:
        """Detect compressed intent (short prompts carrying complex meaning)."""
        return core_detect_compression(text, session_prompt_index)

    def estimate_tm_tier(self, session_data: Dict) -> int:
        """Estimate Orchestra tier from session characteristics."""
        return core_estimate_tm_tier(session_data)

    def calculate_adt_zone(self, dok_adjusted: float, tm_tier: int,
                          trajectory: str | None = None) -> str:
        """Calculate diagnostic zone from DOK x TM matrix."""
        return core_calculate_adt_zone(dok_adjusted, tm_tier, trajectory)

    # --- Data Access ------------------------------------------------------

    def get_data_summary(self) -> Dict:
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT
                MIN(created_at) as earliest,
                MAX(created_at) as latest,
                COUNT(*) as total_sessions
            FROM sessions
            WHERE session_type = 'user'
        """)
        row = cursor.fetchone()

        if not row or not row['earliest']:
            return {'sessions': 0, 'earliest': None, 'latest': None, 'days': 0}

        earliest = datetime.fromisoformat(row['earliest'].replace('Z', '+00:00')) if row['earliest'] else None
        latest = datetime.fromisoformat(row['latest'].replace('Z', '+00:00')) if row['latest'] else None
        days = (latest - earliest).days if earliest and latest else 0

        return {
            'sessions': row['total_sessions'],
            'earliest': row['earliest'],
            'latest': row['latest'],
            'days': days
        }

    def get_all_user_prompts(self) -> List[Dict]:
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT
                m.session_id,
                s.name as session_name,
                s.created_at as session_date,
                s.working_dir,
                m.content_json,
                m.created_timestamp
            FROM messages m
            JOIN sessions s ON m.session_id = s.id
            WHERE m.role = 'user'
                AND s.session_type = 'user'
            ORDER BY m.created_timestamp
        """)

        prompts = []
        for row in cursor.fetchall():
            try:
                content = json.loads(row['content_json'])
                text = None
                for item in content:
                    if isinstance(item, dict) and item.get('type') == 'text':
                        text = item.get('text', '')
                        break

                if text:
                    prompts.append({
                        'session_id': row['session_id'],
                        'session_name': row['session_name'],
                        'session_date': row['session_date'],
                        'working_dir': row['working_dir'],
                        'text': text,
                        'timestamp': row['created_timestamp']
                    })
            except (json.JSONDecodeError, TypeError):
                continue

        return prompts

    def _has_accumulated_columns(self) -> bool:
        """Check if sessions table has accumulated_* token columns."""
        cursor = self.conn.cursor()
        cursor.execute("PRAGMA table_info(sessions)")
        columns = {row['name'] for row in cursor.fetchall()}
        return 'accumulated_total_tokens' in columns

    def get_session_metadata(self) -> Dict[str, Dict]:
        """Get per-session metadata for TM estimation"""
        cursor = self.conn.cursor()
        has_accumulated = self._has_accumulated_columns()

        if has_accumulated:
            cursor.execute("""
                SELECT
                    s.id,
                    s.created_at,
                    s.total_tokens,
                    s.accumulated_total_tokens,
                    s.accumulated_input_tokens,
                    s.accumulated_output_tokens,
                    s.working_dir,
                    COUNT(m.id) as message_count
                FROM sessions s
                LEFT JOIN messages m ON m.session_id = s.id AND m.role = 'user'
                WHERE s.session_type = 'user'
                GROUP BY s.id
            """)
        else:
            cursor.execute("""
                SELECT
                    s.id,
                    s.created_at,
                    s.total_tokens,
                    s.working_dir,
                    COUNT(m.id) as message_count
                FROM sessions s
                LEFT JOIN messages m ON m.session_id = s.id AND m.role = 'user'
                WHERE s.session_type = 'user'
                GROUP BY s.id
            """)

        sessions = {}
        for row in cursor.fetchall():
            total = row['total_tokens'] or 0
            sessions[row['id']] = {
                'created_at': row['created_at'],
                'total_tokens': total,
                'accumulated_tokens': row['accumulated_total_tokens'] or 0 if has_accumulated else total,
                'accumulated_input': row['accumulated_input_tokens'] or 0 if has_accumulated else 0,
                'accumulated_output': row['accumulated_output_tokens'] or 0 if has_accumulated else 0,
                'working_dir': row['working_dir'],
                'prompt_count': row['message_count']
            }

        # Check for tool usage patterns and sub-agent indicators
        # Sub-agent tools are distinguished from general tool usage
        SUBAGENT_TOOL_NAMES = {'delegate', 'subagent', 'spawn', 'fork', 'parallel'}

        cursor.execute("""
            SELECT
                m.session_id,
                m.content_json
            FROM messages m
            JOIN sessions s ON m.session_id = s.id
            WHERE m.role = 'assistant'
                AND s.session_type = 'user'
        """)

        # Collect unique tools per session (across all messages)
        session_tools: Dict[str, set] = defaultdict(set)

        for row in cursor.fetchall():
            session_id = row['session_id']
            if session_id not in sessions:
                continue
            try:
                content = json.loads(row['content_json'])
                for item in content:
                    if isinstance(item, dict):
                        # Goose uses 'toolRequest' with nested toolCall.value.name
                        if item.get('type') == 'toolRequest':
                            tool_call = item.get('toolCall', {})
                            value = tool_call.get('value', {}) if isinstance(tool_call, dict) else {}
                            tool_name = value.get('name', '').lower() if isinstance(value, dict) else ''
                            if tool_name:
                                session_tools[session_id].add(tool_name)
                                if tool_name in SUBAGENT_TOOL_NAMES:
                                    sessions[session_id]['has_subagents'] = True
                        # Also check for Claude/generic format
                        elif item.get('type') in ('toolUse', 'tool_use'):
                            tool_name = item.get('name', '').lower()
                            if tool_name:
                                session_tools[session_id].add(tool_name)
                                if tool_name in SUBAGENT_TOOL_NAMES:
                                    sessions[session_id]['has_subagents'] = True
            except (json.JSONDecodeError, TypeError):
                continue

        for session_id, tools in session_tools.items():
            sessions[session_id]['unique_tools'] = len(tools)
            sessions[session_id]['tools_seen'] = list(tools)

        return sessions

    def get_daily_token_spend(self) -> List[Dict]:
        """Get daily token spend aggregated from accumulated session totals.

        Returns a list of dicts sorted by date, each containing:
          day, sessions, prompts, total_tokens, input_tokens, output_tokens,
          tokens_per_prompt
        """
        cursor = self.conn.cursor()
        has_accumulated = self._has_accumulated_columns()

        if has_accumulated:
            cursor.execute("""
                SELECT
                    date(created_at) as day,
                    COUNT(*) as sessions,
                    SUM(CASE WHEN accumulated_total_tokens > 0
                        THEN accumulated_total_tokens ELSE 0 END) as total_tokens,
                    SUM(CASE WHEN accumulated_input_tokens > 0
                        THEN accumulated_input_tokens ELSE 0 END) as input_tokens,
                    SUM(CASE WHEN accumulated_output_tokens > 0
                        THEN accumulated_output_tokens ELSE 0 END) as output_tokens
                FROM sessions
                WHERE session_type = 'user'
                GROUP BY day
                ORDER BY day
            """)
        else:
            cursor.execute("""
                SELECT
                    date(created_at) as day,
                    COUNT(*) as sessions,
                    SUM(CASE WHEN total_tokens > 0
                        THEN total_tokens ELSE 0 END) as total_tokens,
                    0 as input_tokens,
                    0 as output_tokens
                FROM sessions
                WHERE session_type = 'user'
                GROUP BY day
                ORDER BY day
            """)
        token_rows = {row['day']: dict(row) for row in cursor.fetchall()}

        if has_accumulated:
            cursor.execute("""
                SELECT
                    date(s.created_at) as day,
                    COUNT(m.id) as prompts
                FROM sessions s
                JOIN messages m ON m.session_id = s.id AND m.role = 'user'
                WHERE s.session_type = 'user'
                  AND s.accumulated_total_tokens > 0
                GROUP BY day
            """)
        else:
            cursor.execute("""
                SELECT
                    date(s.created_at) as day,
                    COUNT(m.id) as prompts
                FROM sessions s
                JOIN messages m ON m.session_id = s.id AND m.role = 'user'
                WHERE s.session_type = 'user'
                  AND s.total_tokens > 0
                GROUP BY day
            """)
        prompt_rows = {row['day']: row['prompts'] for row in cursor.fetchall()}

        results = []
        for day, data in token_rows.items():
            prompts = prompt_rows.get(day, 0)
            results.append({
                'day': day,
                'sessions': data['sessions'],
                'prompts': prompts,
                'total_tokens': data['total_tokens'],
                'input_tokens': data['input_tokens'],
                'output_tokens': data['output_tokens'],
                'tokens_per_prompt': (
                    round(data['total_tokens'] / prompts)
                    if prompts > 0 else 0
                ),
            })

        return results

    def get_sessions_by_directory(self) -> Dict[str, Dict]:
        cursor = self.conn.cursor()
        cursor.execute("""
            SELECT
                working_dir,
                COUNT(*) as session_count,
                SUM(total_tokens) as total_tokens
            FROM sessions
            WHERE session_type = 'user'
            GROUP BY working_dir
            ORDER BY session_count DESC
        """)

        return {row['working_dir']: {
            'count': row['session_count'],
            'tokens': row['total_tokens'] or 0
        } for row in cursor.fetchall()}

    # --- Analysis ---------------------------------------------------------

    def analyze_prompts(self, prompts: List[Dict]) -> Dict:
        """Full analysis of a set of prompts.

        Uses multi-signal DOK classification:
        1. System messages are filtered out (DOK 0, not counted)
        2. Keyword matching is the primary signal
        3. When no keywords match, session rolling DOK is used as default
        4. Compression detection adjusts short directives +1 DOK
        """
        dok_counts = {1: 0, 2: 0, 3: 0, 4: 0}
        dok_scores = []
        compressed_count = 0
        adjusted_scores = []
        peak_prompt = None
        peak_dok = 0
        system_messages_filtered = 0

        # Group by session for compression detection and context tracking
        session_prompts = defaultdict(list)
        for prompt in prompts:
            session_prompts[prompt['session_id']].append(prompt)

        for _session_id, s_prompts in session_prompts.items():
            session_dok_running = []  # Rolling DOK within this session

            for idx, prompt in enumerate(s_prompts):
                # Calculate session context DOK (rolling average of prior prompts)
                session_context = (
                    sum(session_dok_running) / len(session_dok_running)
                    if session_dok_running else None
                )

                dok_level, _confidence, _matched = self.classify_dok(
                    prompt['text'], session_context_dok=session_context
                )

                # Filter system messages (DOK 0)
                if dok_level == 0:
                    system_messages_filtered += 1
                    continue

                dok_counts[dok_level] += 1
                dok_scores.append(dok_level)
                session_dok_running.append(dok_level)

                # Compression detection
                is_compressed = self.detect_compression(prompt['text'], idx)
                if is_compressed:
                    compressed_count += 1
                    adjusted_level = min(dok_level + 1, 4)
                else:
                    adjusted_level = dok_level

                adjusted_scores.append(adjusted_level)

                # Track peak
                if adjusted_level > peak_dok or (adjusted_level == peak_dok and peak_prompt is None):
                    peak_dok = adjusted_level
                    peak_prompt = prompt

        total_prompts = sum(dok_counts.values())
        if total_prompts == 0:
            return {'error': 'no_prompts'}

        # Calculate metrics
        avg_dok_raw = sum(dok_scores) / len(dok_scores)
        avg_dok_adjusted = sum(adjusted_scores) / len(adjusted_scores)
        dok_3_4_pct = ((dok_counts[3] + dok_counts[4]) / total_prompts) * 100
        compression_pct = (compressed_count / total_prompts) * 100

        dok_distribution = {
            f'dok{k}': round(v / total_prompts * 100, 1)
            for k, v in dok_counts.items()
        }
        dok_distribution_counts = {f'dok{k}_count': v for k, v in dok_counts.items()}

        return {
            'total_prompts': total_prompts,
            'dok_raw': round(avg_dok_raw, 2),
            'dok_adjusted': round(avg_dok_adjusted, 2),
            'dok_lift': round(avg_dok_adjusted - avg_dok_raw, 3),
            'dok_3_4_pct': round(dok_3_4_pct, 1),
            'compression_pct': round(compression_pct, 1),
            'compressed_count': compressed_count,
            'dok_distribution': dok_distribution,
            'dok_distribution_counts': dok_distribution_counts,
            'peak_prompt': peak_prompt,
            'peak_dok': peak_dok,
        }

    def get_daily_breakdown(self, prompts: List[Dict]) -> List[Dict]:
        """Calculate adjusted DOK scores by day (with compression).

        Aggregates ALL sessions from a given day together. If a day has
        10 different Goose sessions, all 10 are analyzed as one unit.
        Uses session-context DOK default and filters system messages.
        """
        # Group prompts by day and session for compression detection
        daily_by_session = defaultdict(lambda: defaultdict(list))

        for prompt in prompts:
            if prompt.get('session_date'):
                try:
                    date = datetime.fromisoformat(prompt['session_date'].replace('Z', '+00:00'))
                    day = date.strftime('%Y-%m-%d')
                    daily_by_session[day][prompt['session_id']].append(prompt)
                except (ValueError, TypeError):
                    continue

        result = []
        for day in sorted(daily_by_session.keys()):
            adjusted_scores = []
            for _session_id, session_prompts in daily_by_session[day].items():
                session_dok_running = []
                for idx, prompt in enumerate(session_prompts):
                    session_context = (
                        sum(session_dok_running) / len(session_dok_running)
                        if session_dok_running else None
                    )
                    dok_level, _, _ = self.classify_dok(
                        prompt['text'], session_context_dok=session_context
                    )
                    # Skip system messages
                    if dok_level == 0:
                        continue
                    session_dok_running.append(dok_level)
                    is_compressed = self.detect_compression(prompt['text'], idx)
                    adjusted_level = min(dok_level + 1, 4) if is_compressed else dok_level
                    adjusted_scores.append(adjusted_level)

            if adjusted_scores:
                avg = sum(adjusted_scores) / len(adjusted_scores)
                dok3_4 = sum(1 for s in adjusted_scores if s >= 3) / len(adjusted_scores) * 100
                result.append({
                    'date': day,
                    'avg_dok': round(avg, 2),
                    'prompt_count': len(adjusted_scores),
                    'dok3_4_pct': round(dok3_4, 1)
                })

        return result

    def detect_phases(self, daily_scores: List[Dict]) -> List[Dict]:
        """Auto-detect phases based on DOK trajectory shifts"""
        if len(daily_scores) < 7:
            return [{'name': 'Current', 'start': daily_scores[0]['date'] if daily_scores else '',
                     'end': daily_scores[-1]['date'] if daily_scores else '',
                     'sessions': len(daily_scores)}]

        # Simple phase detection: split into roughly equal chunks
        # based on trajectory inflection points
        total_days = len(daily_scores)

        if total_days < 14:
            return [{
                'name': '1. Current',
                'start': daily_scores[0]['date'],
                'end': daily_scores[-1]['date'],
                'sessions': total_days,
                'avg_dok': round(sum(d['avg_dok'] for d in daily_scores) / total_days, 2),
                'dok3_4_pct': round(sum(d['dok3_4_pct'] for d in daily_scores) / total_days, 1)
            }]

        # Split into quarters for phase analysis
        quarter = total_days // 4
        phases = []
        phase_names = ['1. Early', '2. Building', '3. Middle', '4. Recent']

        for i, name in enumerate(phase_names):
            start_idx = i * quarter
            end_idx = (i + 1) * quarter if i < 3 else total_days
            chunk = daily_scores[start_idx:end_idx]

            if chunk:
                phases.append({
                    'name': name,
                    'start': chunk[0]['date'],
                    'end': chunk[-1]['date'],
                    'sessions': len(chunk),
                    'avg_dok': round(sum(d['avg_dok'] for d in chunk) / len(chunk), 2),
                    'dok3_4_pct': round(sum(d['dok3_4_pct'] for d in chunk) / len(chunk), 1)
                })

        return phases

    def calculate_trajectory(self, daily_scores: List[Dict]) -> str:
        if len(daily_scores) < 4:
            return "insufficient_data"

        mid = len(daily_scores) // 2
        first_half = [d['avg_dok'] for d in daily_scores[:mid]]
        second_half = [d['avg_dok'] for d in daily_scores[mid:]]

        first_avg = sum(first_half) / len(first_half) if first_half else 0
        second_avg = sum(second_half) / len(second_half) if second_half else 0

        diff = second_avg - first_avg

        if diff > 0.15:
            return "improving"
        elif diff < -0.15:
            return "declining"
        else:
            return "stable"

    # --- Baseline Generation ----------------------------------------------

    def generate_baseline(self) -> Dict:
        if not self.connect():
            return None

        try:
            summary = self.get_data_summary()
            if summary['sessions'] == 0:
                return {'error': 'no_data', 'message': 'No sessions found in database'}

            prompts = self.get_all_user_prompts()
            analysis = self.analyze_prompts(prompts)

            if 'error' in analysis:
                return {'error': 'no_prompts', 'message': 'No classifiable prompts found'}

            # Get session metadata for TM estimation
            session_meta = self.get_session_metadata()
            directories = self.get_sessions_by_directory()

            # Estimate overall TM tier
            tm_tiers = []
            for _sid, meta in session_meta.items():
                tier = self.estimate_tm_tier(meta)
                tm_tiers.append(tier)
            avg_tm = round(sum(tm_tiers) / len(tm_tiers)) if tm_tiers else 2

            # Daily breakdown for trajectory (needed for ADT zone)
            daily_scores = self.get_daily_breakdown(prompts)
            trajectory = self.calculate_trajectory(daily_scores)

            # Calculate ADT zone (trajectory-aware)
            adt_zone = self.calculate_adt_zone(analysis['dok_adjusted'], avg_tm, trajectory)

            # Calculate floor (10th percentile DOK)
            all_daily_doks = sorted([d['avg_dok'] for d in daily_scores])
            floor_idx = max(0, len(all_daily_doks) // 10)
            floor = all_daily_doks[floor_idx] if all_daily_doks else 1.0

            # Growth targets
            current_dok34 = analysis['dok_3_4_pct']
            dok34_target = min(current_dok34 + 15, 50)
            compression_target = max(analysis['compression_pct'] + 5, 5)

            # Token spend
            daily_token_data = self.get_daily_token_spend()
            total_tokens = sum(d['total_tokens'] for d in daily_token_data)
            total_prompts_for_tokens = sum(d['prompts'] for d in daily_token_data)
            daily_avg_tokens = (
                round(total_tokens / len(daily_token_data))
                if daily_token_data else 0
            )
            tokens_per_prompt = (
                round(total_tokens / total_prompts_for_tokens)
                if total_prompts_for_tokens > 0 else 0
            )

            baseline = {
                'version': '4.0',
                'generated_at': datetime.now().isoformat(),
                'period': {
                    'start': summary['earliest'][:10] if summary['earliest'] else None,
                    'end': summary['latest'][:10] if summary['latest'] else None,
                    'days': summary['days']
                },
                'sessions_analyzed': summary['sessions'],
                'prompts_classified': analysis['total_prompts'],
                'three_dimensions': {
                    'dok_adjusted': analysis['dok_adjusted'],
                    'dok_raw': analysis['dok_raw'],
                    'dok_lift': analysis['dok_lift'],
                    'tm_tier': avg_tm,
                    'tm_name': self.TM_TIERS.get(avg_tm, 'Unknown'),
                    'adt_zone': adt_zone
                },
                'dok_distribution': analysis['dok_distribution'],
                'dok_distribution_counts': analysis['dok_distribution_counts'],
                'dok_3_4_pct': analysis['dok_3_4_pct'],
                'compression_pct': analysis['compression_pct'],
                'floor': round(floor, 2),
                'trajectory': trajectory,
                'token_spend': {
                    'total_tokens': total_tokens,
                    'daily_avg_tokens': daily_avg_tokens,
                    'tokens_per_prompt': tokens_per_prompt,
                },
                'domain_count': len(directories),
                'domains': {k: v for k, v in list(directories.items())[:10]},
                'daily_scores': daily_scores[-30:],  # Last 30 days
                'growth_targets': {
                    'dok_3_4_pct': round(dok34_target, 1),
                    'compression_pct': round(compression_target, 1),
                    'next_tm_tier': min(avg_tm + 1, 6),
                    'next_tm_name': self.TM_TIERS.get(min(avg_tm + 1, 6), 'Virtuoso')
                },
                # Preserve v3 fields for backward compat
                'average_dok_score': analysis['dok_adjusted'],
                'estimated_stage': self._tm_to_gas_town(avg_tm),
            }

            return baseline

        finally:
            self.close()

    def _tm_to_gas_town(self, tm_tier: int) -> int:
        """Map TM tier back to Gas Town stage for backward compat"""
        mapping = {1: 2, 2: 3, 3: 4, 4: 5, 5: 6, 6: 7}
        return mapping.get(tm_tier, 5)

    def _gas_town_to_tm(self, stage: int) -> int:
        """Map Gas Town stage to TM tier (for reading v3 baselines)"""
        if stage <= 2:
            return 1
        elif stage == 3:
            return 2
        elif stage == 4:
            return 3
        elif stage == 5:
            return 4
        elif stage == 6:
            return 5
        else:
            return 6

    # --- Baseline I/O -----------------------------------------------------

    def save_baseline(self, baseline: Dict) -> bool:
        try:
            self._get_baseline_file().parent.mkdir(parents=True, exist_ok=True)
            with open(self._get_baseline_file(), 'w') as f:
                json.dump(baseline, f, indent=2)
            return True
        except IOError as e:
            print(f"Error saving baseline: {e}")
            return False

    def load_baseline(self) -> Optional[Dict]:
        if not self._get_baseline_file().exists():
            return None

        try:
            with open(self._get_baseline_file(), 'r') as f:
                baseline = json.load(f)

            # Migrate v3 baseline to v4 format if needed
            version = baseline.get('version', '3.0')
            version_tuple = tuple(int(x) for x in str(version).split('.'))
            if version_tuple < (4, 0):
                baseline = self._migrate_v3_baseline(baseline)

            return baseline
        except (IOError, json.JSONDecodeError) as e:
            print(f"Error loading baseline: {e}")
            return None

    def _migrate_v3_baseline(self, old: Dict) -> Dict:
        """Translate a v3 (Gas Town x DOK) baseline to v4 (Three Dimensions)"""
        stage = old.get('estimated_stage', 5)
        tm_tier = self._gas_town_to_tm(stage)
        dok = old.get('average_dok_score', 2.0)
        adt_zone = self.calculate_adt_zone(dok, tm_tier)

        # Map old quadrant names to new zone names
        quadrant_map = {
            'Learning Zone': 'Expected',
            'Frontier': 'Frontier',
            'Thinking Ahead': 'Thinking Ahead',
            'Underutilizing': 'Underutilizing',
            'Overpowered': 'Overpowered',
            'Growing': 'Growing'
        }
        old_quadrant = old.get('quadrant', '')
        if old_quadrant in quadrant_map:
            adt_zone = quadrant_map[old_quadrant]

        migrated = {
            'version': '4.0',
            'migrated_from': '3.0',
            'generated_at': old.get('generated_at', ''),
            'period': old.get('period', {}),
            'sessions_analyzed': old.get('sessions_analyzed', 0),
            'prompts_classified': old.get('prompts_classified', 0),
            'three_dimensions': {
                'dok_adjusted': dok,
                'dok_raw': dok,
                'dok_lift': 0.0,
                'tm_tier': tm_tier,
                'tm_name': self.TM_TIERS.get(tm_tier, 'Unknown'),
                'adt_zone': adt_zone
            },
            'dok_distribution': old.get('dok_distribution', {}),
            'dok_distribution_counts': old.get('dok_distribution_counts', {}),
            'dok_3_4_pct': old.get('dok_distribution', {}).get('dok3', 0) + old.get('dok_distribution', {}).get('dok4', 0),
            'compression_pct': 0.0,
            'floor': dok - 0.3,
            'trajectory': old.get('trajectory', 'stable'),
            'domain_count': old.get('domain_count', 0),
            'domains': old.get('domains', {}),
            'growth_targets': {
                'dok_3_4_pct': old.get('growth_targets', {}).get('dok3_percentage_target', 35),
                'compression_pct': 5.0,
                'next_tm_tier': min(tm_tier + 1, 6),
                'next_tm_name': self.TM_TIERS.get(min(tm_tier + 1, 6), 'Virtuoso')
            },
            # Preserve v3 fields
            'average_dok_score': dok,
            'estimated_stage': stage,
        }

        return migrated

    # --- Report Formatting ------------------------------------------------

    def format_bar(self, percentage: float, width: int = 20) -> str:
        filled = int(percentage / 100 * width)
        return '\u2588' * filled + '\u2591' * (width - filled)

    def print_baseline_report(self, baseline: Dict):
        if 'error' in baseline:
            print(f"\n{baseline['message']}")
            return

        sessions = baseline['sessions_analyzed']
        dims = baseline.get('three_dimensions', {})

        # Data volume warning
        if sessions < 5:
            print("\n  LIMITED DATA AVAILABLE")
            print("  " + "-" * 60)
            print(f"  Found only {sessions} sessions. Baseline will improve with more data.")
            print()

        print()
        print("=" * 66)
        print("                    rp-why . BASELINE")
        print("=" * 66)
        print()

        # Data Summary
        print("DATA SUMMARY")
        print("-" * 66)
        period = baseline.get('period', {})
        start = period.get('start', 'unknown')
        end = period.get('end', 'unknown')
        days = period.get('days', 0)
        print(f"Period:           {start} - {end} ({days} days)")
        print(f"Sessions:         {sessions}")
        print(f"Prompts:          {baseline.get('prompts_classified', 0)} classified")
        print()

        # Three Dimensions
        print("THREE DIMENSIONS")
        print("-" * 66)
        dok = dims.get('dok_adjusted', 0)
        tm_tier = dims.get('tm_tier', 2)
        tm_name = dims.get('tm_name', 'Unknown')
        adt_zone = dims.get('adt_zone', 'Unknown')
        print(f"DOK (Depth of Knowledge)     {dok:.2f}")
        print(f"TM  (Tool Maturity)          Tier {tm_tier} . {tm_name}")
        print(f"ADT (Delegation Trust)       {adt_zone}")
        print()

        # Diagnostic Zone
        print(f"DIAGNOSTIC ZONE: {adt_zone}")
        print("-" * 66)
        zone_descriptions = {
            'Frontier': 'TM and DOK matched and growing together. Operating at the\nproductive edge.',
            'Growing': 'Approaching a match between tool sophistication and cognitive\ndepth. Building toward effective use.',
            'Expected': 'Tool usage and cognitive depth appropriate for current level.\nHealthy starting position.',
            'Thinking Ahead': 'Cognitive depth exceeds tool sophistication. Opportunity to\nadopt more powerful orchestration patterns.',
            'Underutilizing': 'Tool sophistication exceeds cognitive depth. Opportunity to\ndeepen the questions being asked.',
            'Overpowered': 'Significant mismatch between tool complexity and task depth.\nResources spent without proportional cognitive return.'
        }
        print(zone_descriptions.get(adt_zone, ''))
        print()

        # DOK Distribution
        print("DOK DISTRIBUTION")
        print("-" * 66)
        dist = baseline.get('dok_distribution', {})
        for i in range(1, 5):
            pct = dist.get(f'dok{i}', 0)
            bar = self.format_bar(pct)
            print(f"DOK {i} ({self.DOK_NAMES_SHORT[i]:11}):  {bar}  {pct:5.1f}%")
        print()
        print(f"DOK 3+4:  {baseline.get('dok_3_4_pct', 0):.1f}%")
        print(f"Compression:  {baseline.get('compression_pct', 0):.1f}%")
        print()

        # Token Spend
        token_data = baseline.get('token_spend', {})
        if token_data.get('total_tokens', 0) > 0:
            total = token_data['total_tokens']
            daily_avg = token_data.get('daily_avg_tokens', 0)
            tpp = token_data.get('tokens_per_prompt', 0)
            print("TOKEN SPEND")
            print("-" * 66)
            print(f"Total:            {total / 1_000_000_000:.2f}B tokens")
            print(f"Daily average:    {daily_avg / 1_000_000:.1f}M")
            print(f"Tokens/prompt:    {tpp:,}")
            print()

        # Growth Targets
        targets = baseline.get('growth_targets', {})
        print("GROWTH TARGETS")
        print("-" * 66)
        print(f"DOK 3+4 target:       {targets.get('dok_3_4_pct', 35):.0f}%")
        print(f"Compression target:   {targets.get('compression_pct', 5):.0f}%")
        next_tier = targets.get('next_tm_tier', tm_tier + 1)
        next_name = targets.get('next_tm_name', '')
        tier_descriptions = {
            2: 'back-and-forth conversation',
            3: 'provide meaningful body of work, evaluate holistically',
            4: 'delegate work streams, introduce sub-agents',
            5: 'coordinate multiple AI interactions toward unified goal',
            6: 'flow state, human and AI synthesized'
        }
        tier_desc = tier_descriptions.get(next_tier, '')
        print(f"Next TM tier:         Tier {next_tier} . {next_name} ({tier_desc})")
        print()

        # Footer
        print("-" * 66)
        print(f"Baseline saved to: {self._get_baseline_file()}")
        print("Run /rp-why current after sessions to track progress.")
        print()

    def print_current_report(self, analysis: Dict, session_meta: Dict | None = None):
        if 'error' in analysis:
            print(f"\n{analysis.get('message', 'No data available')}")
            return

        # Estimate TM from session metadata
        tm_tier = 2  # Default
        if session_meta:
            tm_tier = self.estimate_tm_tier(session_meta)

        dok_adj = analysis['dok_adjusted']
        adt_zone = self.calculate_adt_zone(dok_adj, tm_tier)

        print()
        print("=" * 66)
        print("                    rp-why . CURRENT SESSION")
        print("=" * 66)
        print()

        # Session Snapshot
        print("SESSION SNAPSHOT")
        print("-" * 66)
        print(f"Date:             {datetime.now().strftime('%b %d, %Y')}")
        print(f"Prompts:          {analysis['total_prompts']} classified")
        session_tokens = session_meta.get('accumulated_tokens', 0) if session_meta else 0
        if session_tokens > 0:
            tpp = round(session_tokens / analysis['total_prompts']) if analysis['total_prompts'] > 0 else 0
            print(f"Token spend:      {session_tokens / 1_000_000:.1f}M ({tpp:,}/prompt)")
        print()

        # Three Dimensions
        print("THREE DIMENSIONS")
        print("-" * 66)
        print(f"DOK (Adjusted)       {dok_adj:.2f}")
        print(f"TM  (Tool Maturity)  Tier {tm_tier} . {self.TM_TIERS[tm_tier]}")
        print(f"ADT (Delegation)     {adt_zone}")
        print()

        # Diagnostic Zone
        print(f"DIAGNOSTIC ZONE: {adt_zone}")
        print("-" * 66)
        zone_descriptions = {
            'Frontier': 'TM and DOK matched and growing together. Operating at the\nproductive edge.',
            'Growing': 'Approaching a match. Building toward effective use of the\ncurrent toolset.',
            'Expected': 'Appropriate match for current level. Growth comes from\nasking "why" before implementing.',
            'Thinking Ahead': 'Cognitive depth exceeds tools. Time to adopt more powerful\norchestration patterns.',
            'Underutilizing': 'Powerful tools deserve powerful questions. Opportunity to\ndeepen the questions being asked.',
            'Overpowered': 'Tools exceed task needs. Consider whether this task needs\nan autonomous agent.'
        }
        print(zone_descriptions.get(adt_zone, ''))
        print()

        # DOK Distribution
        print("DOK DISTRIBUTION")
        print("-" * 66)
        dist = analysis['dok_distribution']
        for i in range(1, 5):
            pct = dist.get(f'dok{i}', 0)
            bar = self.format_bar(pct)
            print(f"DOK {i} ({self.DOK_NAMES_SHORT[i]:11}):  {bar}  {pct:5.1f}%")
        print()
        print(f"DOK 3+4:  {analysis['dok_3_4_pct']:.1f}%     Compression:  {analysis['compression_pct']:.1f}%")
        print()

        # Peak Moment
        peak = analysis.get('peak_prompt')
        if peak:
            peak_text = peak['text'][:70] + '...' if len(peak['text']) > 70 else peak['text']
            peak_dok = analysis['peak_dok']
            print("PEAK MOMENT")
            print("-" * 66)
            print(f'"{peak_text}"')
            print(f"  DOK {peak_dok} . {self.DOK_NAMES[peak_dok]}")
            print()

        # Growth Nudge
        print("GROWTH NUDGE")
        print("-" * 66)
        nudges = {
            'Frontier': 'Strong session. The collaboration is matched and productive.\nConsider extending one thread into a multi-session investigation.',
            'Growing': 'Building momentum. Try framing one more task as a design\ndecision rather than an execution request.',
            'Expected': 'Solid foundation. Ask "what are the trade-offs?" before your\nnext implementation prompt.',
            'Thinking Ahead': 'Your thinking exceeds your tools. Try delegating a work\nstream to a sub-agent or multi-step workflow.',
            'Underutilizing': 'Powerful tools available. Before each prompt: can this be\nmore strategic? Batch simple queries.',
            'Overpowered': 'Consider whether this task needs an autonomous agent.\nRedirect toward problems requiring analysis or design.'
        }
        print(nudges.get(adt_zone, ''))
        print()

        # Reflection
        reflections = {
            'Frontier': 'What complex challenge could benefit from sustained\n   exploration across your next few sessions?',
            'Growing': 'What workflow could you delegate more fully to the agent?',
            'Expected': 'What strategic question have you been avoiding?',
            'Thinking Ahead': 'What tool or workflow would unlock the depth you are\n   already thinking at?',
            'Underutilizing': 'What is the most strategic question you could ask right now?',
            'Overpowered': 'Is there a harder problem this tool should be pointed at?'
        }
        reflection = reflections.get(adt_zone, 'What could you explore more deeply?')
        print(f"  {reflection}")
        print()

    def print_compare_report(self, baseline: Dict, current: Dict, session_meta: Dict | None = None):
        if 'error' in current:
            print(f"\n{current.get('message', 'No current data')}")
            return

        dims_b = baseline.get('three_dimensions', {})
        dok_b = dims_b.get('dok_adjusted', 0)
        tm_b = dims_b.get('tm_tier', 2)
        adt_b = dims_b.get('adt_zone', 'Unknown')

        # Current dimensions - use session metadata for TM estimation
        dok_c = current['dok_adjusted']
        if session_meta:
            tm_c = self.estimate_tm_tier(session_meta)
        else:
            tm_c = tm_b
        # Single-day compare cannot measure trajectory - pass None
        adt_c = self.calculate_adt_zone(dok_c, tm_c, None)

        period = baseline.get('period', {})
        period_str = f"{period.get('start', '?')} - {period.get('end', '?')}"

        print()
        print("=" * 66)
        print("                    rp-why . COMPARE")
        print("=" * 66)
        print()
        print(f"COMPARING: Today ({datetime.now().strftime('%b %d')}) vs. Baseline ({period_str})")
        print()

        # Three Dimensions comparison
        print(f"{'THREE DIMENSIONS':<24} {'Baseline':>12} {'Today':>12} {'':>8}")
        print("-" * 66)
        dok_delta = dok_c - dok_b
        dok_pct = (dok_delta / dok_b * 100) if dok_b > 0 else 0
        print(f"{'DOK (Adjusted)':<24} {dok_b:>12.2f} {dok_c:>12.2f} {f'+{dok_pct:.0f}%' if dok_pct >= 0 else f'{dok_pct:.0f}%':>8}")
        print(f"{'TM  (Tool Maturity)':<24} {'Tier ' + str(tm_b):>12} {'Tier ' + str(tm_c):>12} {f'+{tm_c - tm_b}' if tm_c > tm_b else ('=' if tm_c == tm_b else str(tm_c - tm_b)):>8}")
        arrow = '=' if adt_b == adt_c else '>'
        print(f"{'ADT (Delegation Trust)':<24} {adt_b:>12} {adt_c:>12} {'':>8}")
        print()

        # Diagnostic Zone transition
        if adt_b != adt_c:
            print(f"DIAGNOSTIC ZONE: {adt_b} -> {adt_c}")
        else:
            print(f"DIAGNOSTIC ZONE: {adt_c} (sustained)")
        print("-" * 66)
        print()

        # DOK Distribution comparison
        dist_b = baseline.get('dok_distribution', {})
        dist_c = current.get('dok_distribution', {})

        print(f"{'DOK DISTRIBUTION':<24} {'Baseline':>12} {'Today':>12} {'':>8}")
        print("-" * 66)
        for i in range(1, 5):
            pct_b = dist_b.get(f'dok{i}', 0)
            pct_c = dist_c.get(f'dok{i}', 0)
            delta = pct_c - pct_b
            delta_str = f"+{delta:.1f}pp" if delta >= 0 else f"{delta:.1f}pp"
            print(f"DOK {i} ({self.DOK_NAMES_SHORT[i]:11})   {pct_b:>10.1f}% {pct_c:>10.1f}% {delta_str:>8}")
        print()

        dok34_b = baseline.get('dok_3_4_pct', 0)
        dok34_c = current.get('dok_3_4_pct', 0)
        dok34_delta = dok34_c - dok34_b
        comp_b = baseline.get('compression_pct', 0)
        comp_c = current.get('compression_pct', 0)
        comp_delta = comp_c - comp_b

        print(f"{'DOK 3+4':<24} {dok34_b:>10.1f}% {dok34_c:>10.1f}% {f'+{dok34_delta:.1f}pp' if dok34_delta >= 0 else f'{dok34_delta:.1f}pp':>8}")
        comp_label = "emerged" if comp_b == 0 and comp_c > 0 else (f"+{comp_delta:.1f}pp" if comp_delta >= 0 else f"{comp_delta:.1f}pp")
        print(f"{'Compression':<24} {comp_b:>10.1f}% {comp_c:>10.1f}% {comp_label:>8}")
        print()

        # Token Spend comparison
        token_b = baseline.get('token_spend', {})
        tokens_today = session_meta.get('accumulated_tokens', 0) if session_meta else 0
        daily_avg_b = token_b.get('daily_avg_tokens', 0)
        tpp_b = token_b.get('tokens_per_prompt', 0)
        if tokens_today > 0 and daily_avg_b > 0:
            prompts_today = current.get('total_prompts', 1)
            tpp_c = round(tokens_today / prompts_today) if prompts_today > 0 else 0
            tok_delta_pct = ((tokens_today - daily_avg_b) / daily_avg_b * 100) if daily_avg_b > 0 else 0
            print(f"{'TOKEN SPEND':<24} {'Baseline avg':>12} {'Today':>12} {'':>8}")
            print(f"{'  Daily total':<24} {daily_avg_b / 1_000_000:>10.1f}M {tokens_today / 1_000_000:>10.1f}M {f'+{tok_delta_pct:.0f}%' if tok_delta_pct >= 0 else f'{tok_delta_pct:.0f}%':>8}")
            print(f"{'  Tokens/prompt':<24} {tpp_b:>12,} {tpp_c:>12,} {'':>8}")
            print()

        # Trajectory
        print("TRAJECTORY")
        print("-" * 66)
        if dok_delta > 0.1:
            direction = "Improving"
            arrow = "^"
        elif dok_delta < -0.1:
            direction = "Declining"
            arrow = "v"
        else:
            direction = "Stable"
            arrow = "="
        print(f"Direction:  {arrow} {direction}")

        # Signal interpretation
        signals = []
        if dok34_delta > 10:
            signals.append(f"DOK 3+4 jumped {dok34_delta:.0f}pp. Strategic thinking increased significantly.")
        elif dok34_delta > 5:
            signals.append(f"DOK 3+4 up {dok34_delta:.0f}pp. More strategic work in this session.")
        elif dok34_delta < -5:
            signals.append(f"DOK 3+4 down {abs(dok34_delta):.0f}pp. Execution-heavy session (expected for delegation days).")

        if comp_c > 0 and comp_b == 0:
            signals.append("Compression emerged. Short directives now carry complex intent.")
        elif comp_delta > 5:
            signals.append(f"Compression up {comp_delta:.0f}pp. Trust deepening in the collaboration.")

        if signals:
            print(f"Signal:     {signals[0]}")
            for s in signals[1:]:
                print(f"            {s}")
        print()

        # What Shifted
        print("WHAT SHIFTED")
        print("-" * 66)
        shifts = []
        if dok34_delta > 5:
            shifts.append(f"DOK 3+4 share grew {dok34_delta:.0f}pp - more reasoning and design in prompts")
        dok2_b = dist_b.get('dok2', 0)
        dok2_c = dist_c.get('dok2', 0)
        if dok2_b - dok2_c > 10:
            shifts.append(f"DOK 2 share dropped {dok2_b - dok2_c:.0f}pp - work that was explicit is now compressed or delegated")
        if comp_c > comp_b + 3:
            shifts.append("Compression growing - trust made visible in the data")
        if tm_c > tm_b:
            shifts.append(f"TM advanced from {self.TM_TIERS[tm_b]} to {self.TM_TIERS[tm_c]}")
        if adt_b != adt_c:
            shifts.append(f"Diagnostic zone moved: {adt_b} -> {adt_c}")

        if not shifts:
            shifts.append("Session is consistent with baseline patterns")

        for s in shifts:
            print(f"  . {s}")
        print()

        print("-" * 66)
        print("Run /rp-why overall for full longitudinal analysis.")
        print()

    def print_overall_report(self, baseline: Dict, analysis: Dict, daily_scores: List[Dict]):
        if 'error' in analysis:
            print(f"\n{analysis.get('message', 'No data')}")
            return

        dims = baseline.get('three_dimensions', {})
        tm_tier = dims.get('tm_tier', 2)
        dok_adj = analysis['dok_adjusted']
        # Use fresh trajectory from current daily_scores, not stale baseline
        fresh_trajectory = self.calculate_trajectory(daily_scores)
        adt_zone = self.calculate_adt_zone(dok_adj, tm_tier, fresh_trajectory)

        period = baseline.get('period', {})
        start = period.get('start', '?')
        end = period.get('end', '?')
        days = period.get('days', 0)

        # Rolling averages
        first_10 = daily_scores[:10] if len(daily_scores) >= 10 else daily_scores[:len(daily_scores)//2]
        last_10 = daily_scores[-10:] if len(daily_scores) >= 10 else daily_scores[len(daily_scores)//2:]

        first_dok = sum(d['avg_dok'] for d in first_10) / len(first_10) if first_10 else 0
        last_dok = sum(d['avg_dok'] for d in last_10) / len(last_10) if last_10 else 0
        first_dok34 = sum(d['dok3_4_pct'] for d in first_10) / len(first_10) if first_10 else 0
        last_dok34 = sum(d['dok3_4_pct'] for d in last_10) / len(last_10) if last_10 else 0

        # Floor
        all_doks = sorted([d['avg_dok'] for d in daily_scores])
        floor = all_doks[len(all_doks) // 10] if len(all_doks) > 10 else (all_doks[0] if all_doks else 1.0)

        # Peak
        peak_day = max(daily_scores, key=lambda d: d['avg_dok']) if daily_scores else None

        # Phases
        phases = self.detect_phases(daily_scores)

        print()
        print("=" * 66)
        print("                    rp-why . OVERALL")
        print("=" * 66)
        print()
        print(f"FULL DATASET: {start} - {end} ({days} days)")
        print(f"Sessions: {baseline.get('sessions_analyzed', 0)}  |  Prompts: {analysis['total_prompts']}")
        print()

        # Current Standings
        print("CURRENT STANDINGS")
        print("-" * 66)
        print(f"DOK (Adjusted, full mean)    {dok_adj:.2f}")
        print(f"TM  (Tool Maturity)          Tier {tm_tier} . {self.TM_TIERS[tm_tier]}")
        print(f"ADT (Delegation Trust)       {adt_zone}")
        print()
        print(f"DOK 3+4:  {analysis['dok_3_4_pct']:.1f}%     Compression:  {analysis['compression_pct']:.1f}%     Floor:  {floor:.2f}")
        print()

        # Token Spend in overall
        token_data = baseline.get('token_spend', {})
        if token_data.get('total_tokens', 0) > 0:
            total_tok = token_data['total_tokens']
            daily_avg = token_data.get('daily_avg_tokens', 0)
            tpp = token_data.get('tokens_per_prompt', 0)
            print(f"Token spend:  {total_tok / 1_000_000_000:.2f}B total     {daily_avg / 1_000_000:.1f}M/day     {tpp:,}/prompt")
            print()

        # Rolling Average
        print("ROLLING AVERAGE (Last 10 vs. First 10)")
        print("-" * 66)
        dok_delta = ((last_dok - first_dok) / first_dok * 100) if first_dok > 0 else 0
        dok34_delta = last_dok34 - first_dok34
        print(f"{'':24} {'First 10':>12} {'Last 10':>12} {'':>10}")
        print(f"{'Adjusted DOK':<24} {first_dok:>12.2f} {last_dok:>12.2f} {f'+{dok_delta:.1f}%' if dok_delta >= 0 else f'{dok_delta:.1f}%':>10}")
        print(f"{'DOK 3+4 %':<24} {first_dok34:>11.1f}% {last_dok34:>11.1f}% {f'+{dok34_delta:.1f}pp' if dok34_delta >= 0 else f'{dok34_delta:.1f}pp':>10}")
        print()

        # Phase Analysis
        if len(phases) > 1:
            print("PHASE ANALYSIS")
            print("-" * 66)
            print(f"{'Phase':<16} {'Dates':<22} {'Sessions':>8} {'DOK':>6} {'DOK3+4':>8}")
            print(f"{'-----':<16} {'-----':<22} {'--------':>8} {'---':>6} {'------':>8}")
            for phase in phases:
                dates = f"{phase['start']} - {phase['end'][-5:]}"
                print(f"{phase['name']:<16} {dates:<22} {phase['sessions']:>8} {phase['avg_dok']:>6.2f} {phase['dok3_4_pct']:>7.1f}%")
            print()

        # Trajectory
        print("TRAJECTORY")
        print("-" * 66)
        if peak_day:
            print(f"Peak DOK:     {peak_day['avg_dok']:.2f} ({peak_day['date']})")
        print(f"Floor:        {floor:.2f}")
        trajectory = self.calculate_trajectory(daily_scores)
        direction_map = {'improving': '^ Improving', 'declining': 'v Declining', 'stable': '= Stable'}
        print(f"Direction:    {direction_map.get(trajectory, '? Insufficient data')}")
        print()

        # DOK Distribution
        print("DOK DISTRIBUTION (Full Dataset)")
        print("-" * 66)
        dist = analysis['dok_distribution']
        for i in range(1, 5):
            pct = dist.get(f'dok{i}', 0)
            bar = self.format_bar(pct)
            print(f"DOK {i} ({self.DOK_NAMES_SHORT[i]:11}):  {bar}  {pct:5.1f}%")
        print()

        # Growth Story
        print("GROWTH STORY")
        print("-" * 66)
        if trajectory == 'improving':
            if analysis['compression_pct'] > 5:
                print("The collaboration is deepening. Compression has emerged as a")
                print("signal of trust - short directives now carry complex intent.")
                print("The tool didn't get smarter. The relationship deepened.")
            else:
                print("DOK is trending upward. More strategic and extended thinking")
                print("is appearing in the practice. The next signal to watch for is")
                print("compression - when short prompts start carrying complex meaning.")
        elif trajectory == 'declining':
            print("DOK trending down may indicate execution-heavy work where cognitive")
            print("complexity lives in the orchestration structure, not individual")
            print("prompts. Check if TM tier has advanced - that's the complementary signal.")
        else:
            print("Stable DOK with consistent patterns. Growth may be happening in")
            print("dimensions not captured by prompt text alone - orchestration depth,")
            print("delegation trust, or domain breadth.")
        print()

        # Footer
        print("-" * 66)
        print(f"Data source: {self._get_sessions_db()}")
        print("Methodology: DOK keyword classification, compression detection")
        print("(short prompt + established context), adjusted DOK (+1 for")
        print("compressed prompts, capped at 4).")
        print()


def main():
    import sys

    analyzer = RPWhyAnalyzer()

    if len(sys.argv) < 2:
        print("Usage: python rp_why_baseline.py [init|baseline|current|compare|overall|export|show]")
        print()
        print("Commands:")
        print("  init / baseline   Generate baseline from session history")
        print("  current           Analyze current/recent session")
        print("  compare           Compare current to baseline")
        print("  overall           Full longitudinal report")
        print("  export            Export baseline as JSON")
        print("  show              Show current baseline")
        sys.exit(1)

    command = sys.argv[1].lower()

    # Alias: baseline == init
    if command == 'baseline':
        command = 'init'

    if command == 'init':
        print("\nAnalyzing your Goose session history...\n")
        baseline = analyzer.generate_baseline()

        if baseline and 'error' not in baseline:
            analyzer.save_baseline(baseline)
            analyzer.print_baseline_report(baseline)
        elif baseline:
            print(f"\n{baseline.get('message', 'Unknown error')}")
        else:
            print("\nFailed to generate baseline")

    elif command == 'show':
        baseline = analyzer.load_baseline()
        if baseline:
            analyzer.print_baseline_report(baseline)
        else:
            print("\nNo baseline found. Run 'python rp_why_baseline.py init' first.")

    elif command == 'export':
        baseline = analyzer.load_baseline()
        if baseline:
            print(json.dumps(baseline, indent=2))
        else:
            print("\nNo baseline found. Run 'python rp_why_baseline.py init' first.")

    elif command == 'current':
        if not analyzer.connect():
            sys.exit(1)
        try:
            prompts = analyzer.get_all_user_prompts()
            # Get last session's prompts
            if prompts:
                last_session_id = prompts[-1]['session_id']
                session_prompts = [p for p in prompts if p['session_id'] == last_session_id]
                analysis = analyzer.analyze_prompts(session_prompts)
                session_meta = analyzer.get_session_metadata()
                meta = session_meta.get(last_session_id, {})
                analyzer.print_current_report(analysis, meta)
            else:
                print("\nNo prompts found in recent sessions.")
        finally:
            analyzer.close()

    elif command == 'compare':
        baseline = analyzer.load_baseline()
        if not baseline:
            print("\nNo baseline found. Run 'python rp_why_baseline.py init' first.")
            sys.exit(1)

        if not analyzer.connect():
            sys.exit(1)
        try:
            prompts = analyzer.get_all_user_prompts()
            if prompts:
                # Aggregate all sessions from today (multi-session day)
                today = datetime.now().strftime('%Y-%m-%d')
                today_prompts = [
                    p for p in prompts
                    if p.get('session_date', '').startswith(today)
                ]
                if not today_prompts:
                    # Fallback to last session if no prompts from today
                    last_session_id = prompts[-1]['session_id']
                    today_prompts = [p for p in prompts if p['session_id'] == last_session_id]

                current = analyzer.analyze_prompts(today_prompts)
                session_meta = analyzer.get_session_metadata()
                # Combine metadata from all today's sessions
                today_session_ids = {p['session_id'] for p in today_prompts}
                combined_meta = aggregate_session_metadata(session_meta, today_session_ids)
                analyzer.print_compare_report(baseline, current, combined_meta)
            else:
                print("\nNo prompts found in recent sessions.")
        finally:
            analyzer.close()

    elif command == 'overall':
        baseline = analyzer.load_baseline()
        if not baseline:
            print("\nNo baseline found. Run 'python rp_why_baseline.py init' first.")
            sys.exit(1)

        if not analyzer.connect():
            sys.exit(1)
        try:
            prompts = analyzer.get_all_user_prompts()
            if prompts:
                analysis = analyzer.analyze_prompts(prompts)
                daily_scores = analyzer.get_daily_breakdown(prompts)
                analyzer.print_overall_report(baseline, analysis, daily_scores)
            else:
                print("\nNo prompts found.")
        finally:
            analyzer.close()

    else:
        print(f"Unknown command: {command}")
        print("Usage: python rp_why_baseline.py [init|baseline|current|compare|overall|export|show]")
        sys.exit(1)


if __name__ == '__main__':
    main()
