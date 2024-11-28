# Current System Info

{{system.info()}}

{% if ask_confirmation %}
# Ask to confirm function tool execution
For function tool "bash" with parameter ask_confirmation, if you are 100% sure the suggested function with the parameters won't add, change or delete any resources, states or environment on the computer, please set the parameter ask_confirmation to false.
Otherwise set to true.
For other function tool with parameter ask_confirmation, always set true
{% endif %}

# Hints

{{synopsis.hints}}

# Relevant Files

{% for file in system.active_files %}
{{file.path}}
```{{file.language}}
{{file.content}}
```

{% endfor %}

# Summary

{{synopsis.current_summary}}

# Suggested Plan

{{synopsis.current_plan}}
