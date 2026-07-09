# Claude Code prompt — install the background memory/swap monitor

Paste this to Claude Code, running locally in the `tortu-forks/goose` fork
(`/Users/dougdaulton/AOF/work/projects/forks/goose`).

---

There are three new files already sitting in `_tortu/probes/` (written by a
Cowork session, not yet installed or committed to git):

- `_tortu/probes/probe_monitor.sh` — lightweight script, appends one CSV row
  of memory/swap stats to `_tortu/probes/reports/monitor_log.csv` per run.
- `_tortu/probes/com.tortu.goose.probemonitor.plist` — a launchd agent
  definition that runs the script every 15 minutes.
- `_tortu/probes/INSTALL_MONITOR.md` — the install steps (below), written for
  a human running them manually; you can just execute the commands directly.

Please:

1. **Install the launchd agent:**
   ```bash
   mkdir -p ~/Library/LaunchAgents
   cp _tortu/probes/com.tortu.goose.probemonitor.plist ~/Library/LaunchAgents/
   launchctl load ~/Library/LaunchAgents/com.tortu.goose.probemonitor.plist
   ```

2. **Verify it's actually running and producing data:**
   ```bash
   launchctl list | grep tortu
   sleep 5
   cat _tortu/probes/reports/monitor_log.csv
   ```
   If `monitor_log.csv` has a header row but no data row yet, wait ~30s and
   check again (RunAtLoad should fire it immediately, but give it a moment).
   If it never produces a row, check
   `_tortu/probes/reports/monitor_launchd.err.log` for the actual error and
   let Doug know what it says rather than guessing at a fix.

3. **Git status check:** these three probe files plus whatever the monitor
   has started writing under `_tortu/probes/reports/` are currently
   untracked/uncommitted in this repo. `_tortu/probes/reports/` should
   probably be gitignored (it's runtime output, not source — same treatment
   as `_tortu/config/secrets.env`), but `probe_monitor.sh`,
   `com.tortu.goose.probemonitor.plist`, and `INSTALL_MONITOR.md` are worth
   committing since they're the actual deliverable. Check whether
   `_tortu/probes/reports/` is already covered by an existing `.gitignore`
   entry (it should be, alongside the existing `probe_local_inference.sh`
   reports/ dir if that was ever committed) before deciding whether to add
   one.

4. **Report back** (to Doug directly, or drop a short note in
   `_tortu/_inbox/` if that's the established pattern you're already
   following in this repo): confirm the launchd job is loaded, confirm at
   least one CSV row landed, and flag anything that didn't go as expected.

Nothing here touches recipe/patch work or anything else you may have
in flight — this is a standalone, low-risk local-system task (installing a
background job + a git commit), not a code change to Goose itself.
