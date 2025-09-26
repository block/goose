# Autopilot model selector

This is an advanced feature (config of which may change, use with caution for now)
which lets goose automatically rotate through many providers and models based on rules that trigger as part of its work. 

Models can change at any time, and can help (similar to lead/worker) solve persistent issues, get an advanced plan, a second opinion or more. 

`premade_roles.yaml` are the out of the box configurations, which can be used in the `~/.config/goose/config.yaml` like so: 


```yaml
x-advanced-models:
- provider: databricks
  model: goose-gpt-5
  role: reviewer
- provider: anthropic
  model: claude-opus-4-1-20250805
  role: deep-thinker
```

in this case, when there is some complex activity or planning or thining required, it will automatically switch to opus for a while, likewise when code changes have been made, it will use the reviewer model. 

## Use cases

You can do a lead/worker like combo, or you can default to a low cost model and only in some cases use a frontier model. 
You could default to a local model, and only intermittently switch when needed. 

use `--debug` flag if you want to see it logging when it changes.