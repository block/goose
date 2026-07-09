#!/usr/bin/env python3
"""
Goose Skill: rp-why - Three Dimensions of AI Collaboration

Provides DOK assessments, baseline analysis, comparison reports,
and longitudinal analysis directly within Goose conversations.

Commands:
    /rp-why init      - Generate baseline from all session history
    /rp-why baseline  - Same as init
    /rp-why current   - Analyze current session
    /rp-why compare   - Compare current session to baseline
    /rp-why overall   - Full longitudinal report
"""

from __future__ import annotations

from rp_why_baseline import RPWhyAnalyzer
from rp_why_core import aggregate_session_metadata
from typing import Dict
from pathlib import Path
import json
import io
import sys


class RPWhySkill:
    """Goose skill wrapper for the Three Dimensions rp-why analyzer"""

    HISTORY_FILE = Path.home() / ".config" / "goose" / "rp-why-history.json"

    def __init__(self):
        self.analyzer = RPWhyAnalyzer()

    def _load_history(self) -> list:
        if not self.HISTORY_FILE.exists():
            return []
        try:
            with open(self.HISTORY_FILE, 'r') as f:
                return json.load(f)
        except (IOError, json.JSONDecodeError):
            return []

    def _save_history(self, history: list):
        try:
            self.HISTORY_FILE.parent.mkdir(parents=True, exist_ok=True)
            with open(self.HISTORY_FILE, 'w') as f:
                json.dump(history, f, indent=2)
        except IOError:
            pass

    def _add_to_history(self, data: Dict):
        from datetime import datetime
        history = self._load_history()
        data['timestamp'] = datetime.now().isoformat()
        history.append(data)
        self._save_history(history)

    def _capture_output(self, func, *args, **kwargs) -> str:
        old_stdout = sys.stdout
        sys.stdout = buffer = io.StringIO()
        try:
            func(*args, **kwargs)
        finally:
            output = buffer.getvalue()
            sys.stdout = old_stdout
        return output

    def generate_baseline(self) -> str:
        """Generate baseline analysis from all available session history"""
        baseline = self.analyzer.generate_baseline()

        if baseline and 'error' not in baseline:
            self.analyzer.save_baseline(baseline)
            output = self._capture_output(self.analyzer.print_baseline_report, baseline)

            self._add_to_history({
                'source': 'init',
                'dok_adjusted': baseline['three_dimensions']['dok_adjusted'],
                'tm_tier': baseline['three_dimensions']['tm_tier'],
                'adt_zone': baseline['three_dimensions']['adt_zone'],
                'dok_3_4_pct': baseline['dok_3_4_pct'],
                'compression_pct': baseline['compression_pct'],
            })

            return output
        elif baseline:
            return f"\n{baseline.get('message', 'Unknown error')}"
        else:
            return "\nFailed to generate baseline"

    def analyze_current(self) -> str:
        """Analyze the current/most recent session"""
        if not self.analyzer.connect():
            return "\nCould not connect to sessions database."

        try:
            prompts = self.analyzer.get_all_user_prompts()
            if not prompts:
                return "\nNo prompts found in recent sessions."

            last_session_id = prompts[-1]['session_id']
            session_prompts = [p for p in prompts if p['session_id'] == last_session_id]
            analysis = self.analyzer.analyze_prompts(session_prompts)

            if 'error' in analysis:
                return "\nNo classifiable prompts in current session."

            session_meta = self.analyzer.get_session_metadata()
            meta = session_meta.get(last_session_id, {})

            output = self._capture_output(self.analyzer.print_current_report, analysis, meta)

            self._add_to_history({
                'source': 'current',
                'dok_adjusted': analysis['dok_adjusted'],
                'dok_3_4_pct': analysis['dok_3_4_pct'],
                'compression_pct': analysis['compression_pct'],
            })

            return output
        finally:
            self.analyzer.close()

    def compare_to_baseline(self) -> str:
        """Compare today's sessions (all) to saved baseline.

        Aggregates ALL sessions from today into a single day-level report.
        If a day has 10 different Goose sessions, all 10 are analyzed together.
        """
        from datetime import datetime as dt

        baseline = self.analyzer.load_baseline()
        if not baseline:
            return "\nNo baseline found. Run /rp-why init first."

        if not self.analyzer.connect():
            return "\nCould not connect to sessions database."

        try:
            prompts = self.analyzer.get_all_user_prompts()
            if not prompts:
                return "\nNo prompts found in recent sessions."

            today = dt.now().strftime('%Y-%m-%d')
            today_prompts = [
                p for p in prompts
                if p.get('session_date', '').startswith(today)
            ]

            if not today_prompts:
                last_session_id = prompts[-1]['session_id']
                today_prompts = [p for p in prompts if p['session_id'] == last_session_id]

            current = self.analyzer.analyze_prompts(today_prompts)

            if 'error' in current:
                return "\nNo classifiable prompts in today's sessions."

            session_meta = self.analyzer.get_session_metadata()
            today_session_ids = {p['session_id'] for p in today_prompts}
            combined_meta = aggregate_session_metadata(session_meta, today_session_ids)
            output = self._capture_output(self.analyzer.print_compare_report, baseline, current, combined_meta)

            self._add_to_history({
                'source': 'compare',
                'dok_adjusted': current['dok_adjusted'],
                'dok_3_4_pct': current['dok_3_4_pct'],
                'compression_pct': current['compression_pct'],
            })

            return output
        finally:
            self.analyzer.close()

    def overall_report(self) -> str:
        """Full longitudinal report across all sessions"""
        baseline = self.analyzer.load_baseline()
        if not baseline:
            return "\nNo baseline found. Run /rp-why init first."

        if not self.analyzer.connect():
            return "\nCould not connect to sessions database."

        try:
            prompts = self.analyzer.get_all_user_prompts()
            if not prompts:
                return "\nNo prompts found."

            analysis = self.analyzer.analyze_prompts(prompts)
            daily_scores = self.analyzer.get_daily_breakdown(prompts)

            if 'error' in analysis:
                return "\nNo classifiable prompts found."

            return self._capture_output(
                self.analyzer.print_overall_report, baseline, analysis, daily_scores
            )
        finally:
            self.analyzer.close()

    def token_spend_report(self) -> str:
        """Daily token spend report across all sessions."""
        if not self.analyzer.connect():
            return "\nCould not connect to sessions database."

        try:
            daily = self.analyzer.get_daily_token_spend()
            if not daily:
                return "\nNo token data found."

            output = []
            output.append("=" * 70)
            output.append("                    rp-why . TOKEN SPEND")
            output.append("=" * 70)
            output.append("")

            total_tokens = sum(d['total_tokens'] for d in daily)
            total_prompts = sum(d['prompts'] for d in daily)
            total_sessions = sum(d['sessions'] for d in daily)

            output.append("SUMMARY")
            output.append("-" * 70)
            output.append(f"Period:          {daily[0]['day']} to {daily[-1]['day']} ({len(daily)} days)")
            output.append(f"Sessions:        {total_sessions}")
            output.append(f"Prompts:         {total_prompts}")
            output.append(f"Total tokens:    {total_tokens:,} ({total_tokens / 1_000_000_000:.2f}B)")
            output.append(f"Daily average:   {total_tokens / len(daily) / 1_000_000:.1f}M")
            if total_prompts:
                output.append(f"Tokens/prompt:   {total_tokens / total_prompts:,.0f}")
            output.append("")

            output.append("DAILY BREAKDOWN")
            output.append("-" * 70)
            output.append(f"{'Day':<12} {'Sess':>4} {'Prompts':>7} {'Total (M)':>10} {'Tok/Prompt':>11}")
            output.append(f"{'---':<12} {'----':>4} {'-------':>7} {'---------':>10} {'----------':>11}")

            for d in daily:
                tok_m = d['total_tokens'] / 1_000_000
                tpp = f"{d['tokens_per_prompt']:,}" if d['tokens_per_prompt'] > 0 else "-"
                output.append(
                    f"{d['day']:<12} {d['sessions']:>4} {d['prompts']:>7} "
                    f"{tok_m:>9.1f}M {tpp:>11}"
                )

            output.append("")
            output.append("-" * 70)
            output.append("Source: ~/.local/share/goose/sessions/sessions.db")
            output.append("Field:  accumulated_total_tokens (full session spend)")
            output.append("")

            return "\n".join(output)
        finally:
            self.analyzer.close()


# --- Goose skill interface functions ---

def init() -> str:
    """
    Generate baseline analysis from all available Goose session history.

    Usage:
        /rp-why init
        /rp-why baseline

    Returns:
        Baseline report with Three Dimensions (DOK, TM, ADT),
        DOK distribution, compression %, and growth targets.
    """
    skill = RPWhySkill()
    return skill.generate_baseline()


def baseline() -> str:
    """Alias for init. Generate baseline from session history."""
    return init()


def current() -> str:
    """
    Analyze the current/most recent session.

    Usage:
        /rp-why current

    Returns:
        Session report with Three Dimensions, DOK distribution,
        peak moment, and contextual growth nudge.
    """
    skill = RPWhySkill()
    return skill.analyze_current()


def compare() -> str:
    """
    Compare current session to saved baseline.

    Usage:
        /rp-why compare

    Returns:
        Delta report showing movement across all three dimensions,
        DOK distribution shift, trajectory, and narrative interpretation.
    """
    skill = RPWhySkill()
    return skill.compare_to_baseline()


def overall() -> str:
    """
    Full longitudinal report across all sessions.

    Usage:
        /rp-why overall

    Returns:
        Complete growth picture with current standings, rolling averages,
        phase analysis, trajectory, and growth story narrative.
    """
    skill = RPWhySkill()
    return skill.overall_report()


def token_spend() -> str:
    """
    Show daily token spend across all sessions.

    Usage:
        /rp-why token-spend

    Returns:
        Daily token spend table with sessions, prompts, total tokens,
        and tokens-per-prompt efficiency metric.
    """
    skill = RPWhySkill()
    return skill.token_spend_report()


if __name__ == '__main__':
    import sys

    if len(sys.argv) < 2:
        print("Usage: python goose_skill.py [init|baseline|current|compare|overall|token-spend]")
        sys.exit(1)

    command = sys.argv[1].lower()

    if command in ('init', 'baseline'):
        print(init())
    elif command == 'current':
        print(current())
    elif command == 'compare':
        print(compare())
    elif command == 'overall':
        print(overall())
    elif command in ('token-spend', 'tokens'):
        print(token_spend())
    else:
        print(f"Unknown command: {command}")
        print("Usage: python goose_skill.py [init|baseline|current|compare|overall|token-spend]")
        sys.exit(1)
