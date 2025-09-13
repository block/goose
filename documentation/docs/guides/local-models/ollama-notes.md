---
sidebar_position: 2
title: Ollama Recommendations
description: "Goose and Ollama setup and Models"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Bot } from 'lucide-react';


introduction to using Ollama with Goose goes here, link to the other installation guides for Goose as needed

## Setup

Talk about using `GOOSE_TOOLSHIM_OLLAMA_MODEL`, `GOOSE_TOOLSHIM`, `OLLAMA_TIMEOUT`, `OLLAMA_CONTEXT_LENGTH`

## Recommended Models

This list is co-written by our community, and your level of success will vary based on your hardware and platform. We cannot guarantee these will work for every Goose user. Join our [Discord community](https://discord.gg/block-opensource) to join the discussion.

**michaelneale/qwen3:latest**
- link to ollama page
- RAM/VRAM needed
- core features
- what does it do well
- what does it NOT do well
- recommended settings:
    - OLLAMA_TIMEOUT: 300
    - GOOSE_TOOLSHIM: true

**gpt-oss**
- link to ollama page
- RAM/VRAM needed for :20b and :120b variations
- core features
- what does it do well
- what does it NOT do well
- recommended settings:

**qwen2.5-coder:32b**
- link to ollama page
- RAM/VRAM needed
- core features
- what does it do well
- what does it NOT do well
- recommended settings:
    - OLLAMA_TIMEOUT: 1800
    - GOOSE_TOOLSHIM: true
    - GOOSE_TOOLSHIM_OLLAMA_MODEL: michaelneale/qwen3:latest
