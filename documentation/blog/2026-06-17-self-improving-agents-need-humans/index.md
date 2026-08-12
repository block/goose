---
title: "Self-Improving Agents Still Need Humans"
description: "Benchmarks are most useful when they become bug reports. Here's how the goose team uses Terminal-bench failures, Harbor, and human judgment to improve real agent behavior."
authors:
  - douwe
image: /img/blog/self-improving-agents-need-humans.png
---

![A human engineer reviews an AI agent feedback loop across benchmark dashboards and terminal logs](/img/blog/self-improving-agents-need-humans.png)

[Goodhart's law](https://en.wikipedia.org/wiki/Goodhart%27s_law) is the benchmarker's curse: when a measure becomes a target, it stops being a good measure. Coding-agent benchmarks are almost designed to trigger it. The tasks are public, the result is one number, and the leaderboard inevitably fills up with harnesses that are, often without meaning to be, overfit to the benchmark.

That does not make the go-to standard [Terminal-bench](https://www.tbench.ai/) useless, but it does change how the goose team uses it. The leaderboard is a noisy measure of general agent ability. The signal is a pattern of failures: places where goose keeps getting stuck or where goose fails and another harness succeeds.

That is also why we usually benchmark with [Sonnet](https://www.anthropic.com/claude/sonnet) rather than the strongest model available. We are not trying to get the largest possible number. We want enough failures left on the table to see what support the agent is missing.

<!-- truncate -->

Self-improving agents are all the rage, but the version we trust requires humans for now. The loop is to run the benchmark, have [goose](https://github.com/aaif-goose/goose) compare a task where one harness succeeded and another failed, and ask it to explain the difference in concrete terms. A human then looks across a few of those failures, decides what the general lesson is, and asks goose to implement that broader improvement.

The human step keeps the loop from collapsing into benchmark tricks. Without it, self-improving agents might choose to just write one [Skill](https://agentskills.io/) per task; agents are just as lazy as humans. With it, we can hopefully turn a failure on one task into a capability that should help on tasks we have not seen yet.

The tooling for this lives in [`evals/harbor`](https://github.com/aaif-goose/goose/pull/9637) in the goose repo. The main entry point is `./evals/harbor/cmd.py`, a little Python command with subcommands for `run`, `list`, `show`, `task`, `compare`, and `pull`. It wraps [Harbor](https://github.com/harbor-framework/harbor) around a particular goose binary, model, and set of extensions, then leaves a directory full of per-task JSON and logs.

This is a place where Python works really well. `cmd.py` is a [PEP 723](https://peps.python.org/pep-0723/) [`uv`](https://docs.astral.sh/uv/) script, so the dependencies sit at the top of the file. There is no packaging ceremony, no new repo, and no remembering which virtualenv was blessed. You ask an agent to add a subcommand, instrument a value, or make the output table less terrible, and it just does it. The script becomes both the control surface and the documentation of the experiment. This is where vibe coding shines: small tools that express what needs to be done and where it really doesn't matter if they are coded elegantly.

The actual benchmark runs happen on a remote Linux machine, because a full run is slow, parallel, and Docker-heavy work. But the analysis does not have to stay there. `cmd.py pull tbench@...:/path/to/goose` rsyncs the run directories back down and then the local tool can list runs, inspect one task, or compare two jobs. That matters more than it sounds. If every question requires SSHing into the benchmark box and spelunking through logs by hand, you ask fewer questions.

The comparison recipes in [PR #9637](https://github.com/aaif-goose/goose/pull/9637) implement the step where goose looks at the result. They read two runs for the same task, the agent trajectories, the verifier output, and the reference solution. The useful output explains the mechanism: "A noticed the image, B never opened it" or "A stopped after the correct file existed, B kept rewriting it until it broke."

Here are two findings from recent work. First, goose would sometimes keep exploring after it had enough information to finish. It would be near the answer, but instead of concluding, it would keep poking and eventually run out of turns, failing the benchmark. The same thing can happen in a normal conversation: if goose does not know how far it is into the exchange, it has no real reason to stop exploring and steer toward a solution. [PR #9636](https://github.com/aaif-goose/goose/pull/9636) added turn count awareness to MoIM, the context goose injects to keep the model oriented. So now goose knows when it is time to call it a day.

Second, goose had lost the ability to read images from disk. Originally, when you dragged an image into a goose conversation, we inserted the path to that image and gave the model a tool to read it. That was clunky, so we replaced it with proper image loading in the conversation. In the process, we accidentally deleted the disk image tool too. In the benchmark this showed up as goose using [PIL](https://pillow.readthedocs.io/) to inspect an image instead of using the visual abilities of the model. The comparison recipe made the failure obvious because the successful run looked at the image and the failing run worked around it.

Thanks to the loop, we found two real weaknesses in goose and fixed them. The benchmark improved, which is nice, but more importantly, both fixes should make goose better in normal use: less likely to drift when it should finish and able again to look at an image file sitting on disk.

That is when a benchmark is useful: when it stops being a leaderboard and starts being a bug report.
