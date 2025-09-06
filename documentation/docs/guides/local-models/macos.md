---
sidebar_position: 6
title: Operating System Specifics - MacOS - all Providers
description: "Operating System Specifics - MacOS - all Providers"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Bot } from 'lucide-react';


## Overview

Depending on your processor & to get the best performance, you may need to __[inform your Mac that it is allowed to spin it's fans fast](https://support.apple.com/en-gb/101613)__.

Another consideration is the amount of shared RAM the GPU has access to. Since a while now, this has been runtime configurable, with a caveat being you need __[sudo](https://www.sudo.ws/)__ permissions on your local hardware. Other community links to this information include __[poweruser](https://poweruser.forum/post/6832)__ or __[the referenced GitHub comment](https://github.com/ggml-org/llama.cpp/discussions/2182#discussioncomment-7698315)__, but regardless your information source, you want to be at least aware of the below command.

```
sudo sysctl iogpu.wired_limit_mb=<you need to work out an appropriate number>
```

__NOTE:__ On macOS Sonoma (14.x) and later, the command `sysctl iogpu.wired_limit_mb` is the correct one. However, on older macOS versions like Ventura (13.x), the correct command is `sysctl debug.iogpu.wired_limit`. The older command also takes its value in __bytes__, whereas the newer one takes it in __megabytes__, so be careful with that.

__NOTE:__ Local system stability is yours to manage now.

__NOTE:__ After changing this setting, it is necessary to restart Ollama for it to react to the change.

The above command automatically resets during system boot. If you want to persist this across reboots, it is left as an exercise for the reader. This is deliberate, because you can persistently, significantly and negatively impact the working of your Mac by mis-configuring using sysctl. 
