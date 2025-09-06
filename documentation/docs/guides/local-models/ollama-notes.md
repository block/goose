---
sidebar_position: 1
title: Using Ollama with Goose - Start Here
description: "Using Ollama with Goose - Start Here"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Bot } from 'lucide-react';

## Objective

By the time you finish reading, you should have:
- Clear steps to get `Ollama` + `Goose` + `qwen2.5-coder:32b` calling tools locally on your choice of Operating System.
- A working configuration from which you can evaluate other models locally
- A basis from which to evaluate using, or not, the __[Experimental Ollama Tool Shim](../../experimental/ollama)__


## Prerequisites

This page anticipates that you have:

- Familiarized yourself the relevant __Operating System Specifics__ page, if one exists for your OS, see __[here](./index.md)__
    - Initially, settings mentioned here may be optional, however it is worth being aware of them before continuing
- Followed the __Ollama on__ page for your OS. Again see __[here](./index.md)__
- Goose installed using the __[Install Goose](../../getting-started/installation)__ guide
    - This guide was written using __[1.7.0](https://github.com/block/goose/releases/tag/v1.7.0)__
    - Functionality can change over time: if the guide does not work, you may want to experiment with the Goose version



## Example configuration - this has been seen to work

# TBD - Ian: Model pinning
__as a blog post this wasn't such a big issue, as an official guide it is more so: The existing instructions use model id's that are not pinned. As a consequence third parties can change what is installed over time, increasing the risk this guide is non deterministic.__

Options:

(my assumption is that if this is done on ollama side it will result in a consistent sha dependency chain. I haven't actually validated this. An alternative might be to use hugging face repositories instead, but that seems like an over-complication.)

- no-one does it, and we call it out as a risk in the guide (this is important, because most of the point of this guide is to give the consumer something that is very likely to work)
- I fork the models on ollama and create static names that will always deliver the same model
- Someone in Block does it

__My preference is for someone in Block to do that, and failing that, the first option where we make the consumer aware__



We use the __[Experimental Ollama Tool shim](../../experimental/ollama)__
```
GOOSE_TOOLSHIM=1
```

We will not use the default Tool shim model. Instead we will use one created by one of the Block team.
```
GOOSE_TOOLSHIM_OLLAMA_MODEL=michaelneale/qwen3:latest
```

We will increase the Ollama timeout to 30 minutes - the rationale being: if you already waited 10 mins (default timeout) for a request to complete, the chances are you don't mind waiting up to 30 minutes. This setting should be adjusted per operator preference.
```
OLLAMA_TIMEOUT=1800
```

We use qwen2.5-coder:32b

Meaning __we need to perform__:
```
ollama pull qwen2.5-coder:32b
```
See [https://ollama.com/library/qwen2.5-coder](https://ollama.com/library/qwen2.5-coder)

__and__:
```
ollama run michaelneale/qwen3
```
See [https://ollama.com/michaelneale/qwen3](https://ollama.com/michaelneale/qwen3)

before attempting to launch Goose.


## Configuring Goose

We __[remember](../config-file)__ the configuration file is at `~/.config/goose/config.yaml`

And that if we choose to, we can set all these configuration values for Ollama use at the root of that file. If doing so, the specific values to set there being:
```
OLLAMA_TIMEOUT: 1800
GOOSE_TOOLSHIM: true
GOOSE_TOOLSHIM_OLLAMA_MODEL: michaelneale/qwen3:latest
```


## Launching Goose

Instead of / as well as setting the necessary values in the config file, we remember __[it is also possible to pass (override)](../environment-variables#notes)__ these as `ENV VARS`, for example:

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    ```
    # all one line, wrapped...
    OLLAMA_TIMEOUT=1800 GOOSE_TOOLSHIM=true GOOSE_TOOLSHIM_OLLAMA_MODEL=michaelneale/qwen3:latest open -a <path/to/>goose.app
    ```

    Or, if you elected to set the values in `~/.config/goose/config.yaml` earlier, you can simply start the Goose Desktop App in the MacOS UI.

    The `ENV VARS` approach can be useful when experimenting with multiple models, but is up to the preference of the user.
  </TabItem>

  <TabItem value="cli" label="Goose CLI">
    ```
    # all one line, wrapped...
    OLLAMA_TIMEOUT=1800 GOOSE_TOOLSHIM=true GOOSE_TOOLSHIM_OLLAMA_MODEL=michaelneale/qwen3:latest <path/to/goose-cli-binary/>goose session
    ```

    Or, if you elected to set the values in `~/.config/goose/config.yaml` earlier, you can:
    ```
    <path/to/goose-cli-binary/>goose session
    ```

  </TabItem>
</Tabs>


## Recommended Models
See also: __[Recommended Models](./recommended-models.md)__

## Observing Ollama Memory & GPU usage

With sufficient memory available all models will be 100% on GPU, as seen here:
```
ollama ps
NAME                         ID              SIZE      PROCESSOR    CONTEXT    UNTIL
michaelneale/qwen3:latest    3a38aca461cc    9.6 GB    100% GPU     32768      4 minutes from now
qwen2.5-coder:32b            b92d6a0bd47e    27 GB     100% GPU     32768      2 minutes from now
qwen2.5-coder:14b            9ec8897f747e    15 GB     100% GPU     32768      3 minutes from now
qwen2.5-coder:latest         dae161e27b0e    8.0 GB    100% GPU     32768      4 minutes from now
gpt-oss:20b                  aa4295ac10c3    18 GB     100% GPU     32768      4 minutes from now
qwen3-coder:30b              ad67f85ca250    23 GB     100% GPU     32768      4 minutes from now
gpt-oss:120b                 f7f8e2f8f4e0    74 GB     100% GPU     32768      4 minutes from now
```
With a sharp eye, the reader will notice the above list is faked - MacBooks don't have that much RAM.

In a more constrained memory environment, which below I've simulated using:
```
sudo sysctl iogpu.wired_limit_mb=12400
```

You may see the model being split across the CPU + GPU:
```
ollama ps
NAME                 ID              SIZE     PROCESSOR          CONTEXT    UNTIL
qwen2.5-coder:32b    b92d6a0bd47e    27 GB    53%/47% CPU/GPU    32768      4 minutes from now
```
This is undesirable, and will significantly & negatively impact performance. Your mission, is to manage the combination of model size(s) + available system memory, such that all models used fit into GPU memory.