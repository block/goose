---
sidebar_position: 7
title: Recommended Models
description: "Recommended Models"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Bot } from 'lucide-react';

This list is co-written by our community, and your level of success will vary based on your hardware and platform. We cannot guarantee these will work for every Goose user. Join our [Discord community](https://discord.gg/block-opensource) to join the discussion.

It is worth remembering there has been prior analysis: __[Community-Inspired Benchmarking: The Goose Vibe Check](https://block.github.io/goose/blog/2025/03/31/goose-benchmark/)__, with `qwen2.5-coder:32b` being the highest scoring Ollama hosted model. At the time of writing, local re-tests using goose-bench have not been completed.

In the model list below, `SIZE` = `GB`

**michaelneale/qwen3:latest**
- [link to ollama page](https://ollama.com/michaelneale/qwen3)
- RAM/VRAM needed
    ```
    ollama ps michaelneale/qwen3 | awk 'NR==1{print $1,$3; next}{print $1,$3}' | column -t
    NAME                       SIZE
    michaelneale/qwen3:latest  9.6
    ```
- what does it do well
    - fulfil `GOOSE_TOOLSHIM_OLLAMA_MODEL` 
- what does it NOT do well
    - n/a
- see also
    - [Goose and Qwen3 for Local Execution](https://block.github.io/goose/blog/2025/05/12/local-goose-qwen3/)


**gpt-oss:20b**
- [link to ollama page](https://ollama.com/library/gpt-oss:20b)
- RAM/VRAM needed
    ```
    ollama ps gpt-oss:20b | awk 'NR==1{print $1,$3; next}{print $1,$3}' | column -t
    NAME         SIZE
    gpt-oss:20b  18
    ```
- use in conjunction with toolshim
    - yes
- what does it do well
    - anec-data-ly quick to respond compared with other similar sized models
- what does it NOT do well
    - like qwen2.5-coder:32b, has been seen to get stuck composing shell commands
- see also
    - n/a

**gpt-oss:120b**
- [link to ollama page](https://ollama.com/library/gpt-oss:120b)
- RAM/VRAM needed
    ```
    ollama ps gpt-oss:120b | awk 'NR==1{print $1,$3; next}{print $1,$3}' | column -t
    NAME          SIZE
    gpt-oss:120b  74
    ```
- use in conjunction with toolshim
    - yes
- what does it do well
    - anec-data-ly quick to respond compared with other similar sized models. "quick" in this context means sub two minute responses, vs 4+ minute thinking time on MacBook Pro/Max/128GB type hardware
- what does it NOT do well
    - n/a
- see also
    - n/a

**qwen2.5-coder:32b**
- [link to ollama page](https://ollama.com/library/qwen2.5-coder:32b)
- RAM/VRAM needed
    ```
    ollama ps qwen2.5-coder:32b | awk 'NR==1{print $1,$3; next}{print $1,$3}' | column -t
    NAME               SIZE
    qwen2.5-coder:32b  27
    ```
- use in conjunction with toolshim
    - yes
- what does it do well
    - multi step activities generally progress to completion without babysitting
- what does it NOT do well
    - anec-data-ly slower to respond
    - may need additional support composing shell commands. This may be a consequence of Goose tool shim interaction
- see also
    - n/a
   
**qwen3-coder:30b**
- [link to ollama page](https://ollama.com/library/qwen3-coder:30b)
- RAM/VRAM needed
    ```
    ollama ps qwen3-coder:30b | awk 'NR==1{print $1,$3; next}{print $1,$3}' | column -t
    NAME               SIZE
    qwen3-coder:30b    23 GB
    ```
- use in conjunction with toolshim
    - yes
- what does it do well
    - anec-data-ly responsive / faster to reply vs other models of a similar size
- what does it NOT do well
    - potentially has a relationship with context size, but has been seen to give up multi-step actions
- see also
    - n/a

**qwq:32b**
- [link to ollama page](https://ollama.com/library/qwq:32b)
- RAM/VRAM needed

- use in conjunction with toolshim
    - yes
- use in conjunction with toolshim
    - yes
- what does it do well
    - tbd
- what does it NOT do well
    - tbd
- see also
    - n/a