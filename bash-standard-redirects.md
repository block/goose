Below is a concise but thorough reference for how stdout/stderr, tee, redirection and logging work in bash scripts — with examples you can copy/paste. It covers common patterns (duplicate to file+console, capture stderr separately, preserve exit codes, add timestamps, buffering and color considerations), explains pitfalls (order of redirections, pipe exit codes), and gives recommended best-practices you can use in your project.

### 1) Quick primer: file descriptors
- 0 = stdin
- 1 = stdout
- 2 = stderr
- You can create custom fds (3,4,...) with exec if needed.

### 2) Basic redirections
- stdout to file (overwrite): command > out.log
- stdout append: command >> out.log
- stderr to file: command 2> err.log
- stderr append: command 2>> err.log
- both stdout+stderr to same file (Bourne/Bash portable):
  command > all.log 2>&1
  (order matters: 2>&1 must follow the stdout redirection)
- bash shortcut (not portable to sh): command &> all.log

### 3) tee — copy stdout to console and file
- Write stdout to console and file (overwrite):
  command | tee out.log
- Append:
  command | tee -a out.log
- If you want both stdout and stderr captured into a single tee:
  command 2>&1 | tee combined.log
  This sends stderr into stdout, and tee duplicates that stream.

### 4) Tee for stderr separately (process substitution), keeping stderr on stderr
- To capture stderr to a file while still sending it to the console’s stderr:
  command 2> >(tee -a err.log >&2)
- To capture stdout and stderr to separate files and still see both:
  command > >(tee -a out.log) 2> >(tee -a err.log >&2)
- Note: process substitution >( ... ) is a Bash (and ksh/zsh) feature — not POSIX sh.

### 5) Preserve the exit code of the first command in a pipeline
- Problem: by default a pipeline’s exit status is the exit status of the last command. So:
  cmd1 | tee out.log
  returns the exit code of tee, not cmd1.
- Solutions:
  - Use set -o pipefail so the pipeline returns non-zero if any element fails:
    set -o pipefail
    cmd1 | tee out.log
  - Or inspect PIPESTATUS (bash):
    cmd1 | tee out.log
    rc=${PIPESTATUS[0]}   # rc of cmd1
- Best practice in scripts that must detect errors: set -euo pipefail at top (with caution):
  set -euo pipefail
  # then handle specific commands as needed
  # or check ${PIPESTATUS[@]} after a pipeline

### 6) Redirect all script output (global exec)
- To make every command after this point have stdout and stderr duplicated to files and console:
  exec > >(tee -a "$LOG_DIR/script.out.log") 2> >(tee -a "$LOG_DIR/script.err.log" >&2)
- Example with combined single file:
  exec > >(tee -a "$LOG_DIR/script.log") 2>&1
- This is convenient for wrappers that call many commands and you want unified logging.

### 7) Add timestamps to lines
- If you want per-line timestamps, use ts from moreutils:
  cmd | ts '[%Y-%m-%d %H:%M:%S]' | tee logfile
- Or use awk:
  cmd | awk '{ print strftime("%Y-%m-%d %H:%M:%S"), $0; fflush(); }' | tee logfile
- If the producing process buffers output (does not line-buffer when piped), you may not see timestamps in real time (see buffering next).

### 8) Line buffering and lost realtime behavior
- Many programs switch to block buffering when stdout is not a terminal, meaning output appears in bigger chunks (or delayed).
- To force line-buffering:
  - Use stdbuf (GNU coreutils):
    stdbuf -oL -eL cmd | ts ... | tee logfile
  - Or use unbuffer (from expect) / script wrappers. Example:
    unbuffer cmd | tee log
- Note: stdbuf/unbuffer may not work for all programs (some use internal buffering or spawn sub-threads).

### 9) Colors / ANSI sequences
- Many programs turn off color when stdout is not a terminal. You can:
  - Force color with CLI flag (e.g., --color=always)
  - Use unbuffer / stdbuf to keep terminal-like behavior, but be careful.
- To strip ANSI color codes in logs:
  cmd | sed -r "s/\x1B\[[0-9;]*[mK]//g" | tee logfile

### 10) Logging to syslog / systemd
- Use logger to send lines to syslog:
  cmd | while IFS= read -r line; do logger -t myscript "$line"; done
- For systemd, you can use systemd-cat:
  cmd | systemd-cat -t myservice

### 11) Advanced: tee both stdout and stderr into a single file while preserving separate streams
- A pattern to write both streams to same file but keep stderr on stderr:
  cmd > >(tee -a combined.log) 2> >(tee -a combined.log >&2)
- This writes to combined.log twice, but keeps console output types separate.

### 12) Custom file descriptors
- Open a file descriptor for append:
  exec 3>>my.log
  echo "info" >&3
- Useful for writing structured logs from multiple places without mixing with stdout/stderr.

### 13) Examples you can use for sub-recipe runner (practical)
- Quiet child (send only final JSON on stdout, minimal stderr): set child env:
  RUST_LOG=error goose run --recipe "$path" --no-session 2> >(tee -a "$LOG_DIR/${name}.err" >&2) | tee -a "$LOG_DIR/${name}.out"
- Capture both but preserve exit codes and timestamp:
  set -o pipefail
  stdbuf -oL -eL goose run --recipe "$path" --no-session \
    2> >(ts '[%Y-%m-%d %H:%M:%S]' | tee -a "$LOG_DIR/${name}.err" >&2) \
    | ts '[%Y-%m-%d %H:%M:%S]' | tee -a "$LOG_DIR/${name}.out"
  rc=${PIPESTATUS[0]}   # rc from goose
- Redirect everything for the whole script (global):
  LOG_DIR=/var/log/myrun; mkdir -p "$LOG_DIR"
  exec > >(ts '[%Y-%m-%d %H:%M:%S]' | tee -a "$LOG_DIR/script.log") 2>&1
  # everything after this will be logged

### 14) Common pitfalls and gotchas
- Order of redirections matters: command > out 2>&1 works. command 2>&1 > out does not do the same.
- Pipe exit code: pipelines return the last command’s exit code unless you set -o pipefail.
- try_send-style problems: if a logging/notifier channel is bounded, bursts may drop messages; tee will block (it writes synchronously), so if you need non-blocking logging, you must architect accordingly.
- Duplicate writes: using two process substitutions that append to the same file can interleave lines from different streams — use centralized logging or a single combined stream to preserve order.
- Portability: process substitution >(cmd) is a bashism. For /bin/sh compatibility, use temporary files or named pipes.

### 15) Best-practices recommendations
- In scripts used as wrappers around commands that emit lots of logs:
  - set -o pipefail (and optionally set -e with care)
  - use exec > >(tee -a out.log) 2> >(tee -a err.log >&2) early for consistent behavior
  - if child processes are chatty, set RUST_LOG=error (or the child’s quiet flag) to reduce verbosity
  - use stdbuf -oL -eL for line buffering if you need immediate per-line logging
  - use timestamps (ts/awk) on the logging pipeline, not inside the child
  - when capturing both stdout and stderr in one file while keeping console behavior, prefer piping stderr into stdout (2>&1) and tee once to avoid race conditions

If you want, I can:
- Inspect the existing bash scripts in your repo and suggest specific one-line edits to produce quieter logs or to ensure the parent process's notifier isn’t overloaded.
- Produce a small wrapper template (script) that spawns sub-recipes with sensible defaults: RUST_LOG=error, stdbuf, timestamped tee files, and correct exit-code propagation.

Which would you prefer: repository-specific edits or a standalone wrapper script example you can drop in?
