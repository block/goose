# Installing the background memory/swap monitor

`probe_monitor.sh` is a lightweight companion to `probe_local_inference.sh` —
instead of one snapshot, it appends a single CSV row every 15 minutes to
`_tortu/probes/reports/monitor_log.csv`, so you build up a real time-series of
free RAM, swap usage, and which inference-related processes are running. Over
a few days that answers "is this a chronic daily-driver overload, or does it
spike around specific things I do."

This has to be installed from a **real Terminal on your Mac**, not through
Cowork — the sandbox this session runs in can write files into your connected
folders, but it can't register a background job with your Mac's actual
`launchd`. macOS's modern equivalent of a cron job is a launchd agent, which
handles sleep/wake more gracefully than plain `cron`, so that's what this
uses.

## 1. Install the launchd agent

```bash
mkdir -p ~/Library/LaunchAgents
cp /Users/dougdaulton/AOF/work/projects/forks/goose/_tortu/probes/com.tortu.goose.probemonitor.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.tortu.goose.probemonitor.plist
```

That's it — it fires once immediately (`RunAtLoad`), then every 15 minutes
(`StartInterval = 900` seconds) for as long as you're logged in, including
across reboots (launchd re-loads agents from `~/Library/LaunchAgents/` at
login automatically — you don't need to `load` it again after a restart).

## 2. Verify it's running

```bash
launchctl list | grep tortu
```

You should see a line with `com.tortu.goose.probemonitor` and a PID or a
recent exit status (`0` = last run succeeded). Then check that rows are
actually landing:

```bash
tail -5 /Users/dougdaulton/AOF/work/projects/forks/goose/_tortu/probes/reports/monitor_log.csv
```

If nothing shows up after a few minutes, check the launchd-level logs (these
capture things like "script not found" or permission errors, separate from
anything the script itself would print):

```bash
cat /Users/dougdaulton/AOF/work/projects/forks/goose/_tortu/probes/reports/monitor_launchd.err.log
```

## 3. Stop or remove it later

```bash
launchctl unload ~/Library/LaunchAgents/com.tortu.goose.probemonitor.plist
rm ~/Library/LaunchAgents/com.tortu.goose.probemonitor.plist
```

Unloading stops future runs; it doesn't touch the CSV log that's already
been collected.

## 4. Reading the data later

`monitor_log.csv` columns: `timestamp, free_ram_gb, swap_used_gb,
swap_total_gb, swap_pct, mem_free_pct, load_avg_1m, ollama_running,
omlx_running, chrome_running, claude_desktop_running, top_mem_proc`.

Once you've got a few days of rows, hand the CSV back to Cowork/Claude and
ask for the pattern read — e.g. does `swap_pct` climb steadily over a session
and only reset after a reboot, does it correlate with `chrome_running`, does
it spike specifically when `omlx_running`/`ollama_running` flips to yes, etc.
Fifteen-minute granularity over a couple of days is enough to see that shape
without the log file getting unwieldy (roughly 100 rows/day, a few KB).

## Why not literal `cron`

Plain `crontab -e` would technically work too, but on macOS `cron` jobs don't
reliably fire if the Mac was asleep at the scheduled minute — they just get
skipped, which would leave gaps in exactly the kind of over-time pattern
you're trying to see. `launchd` is Apple's supported replacement and handles
that case (catches up shortly after wake), so that's what this uses. If you
already have a `cron` habit and would rather use it, the script itself
doesn't care how it's invoked — `bash probe_monitor.sh` from a crontab line
works identically, you'd just lose the sleep/wake robustness.
