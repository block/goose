# Decentralized Inference with Goose

Run large language models split across multiple devices using llama.cpp's RPC backend. Pool GPU/CPU resources from multiple machines to run models that don't fit on a single device.

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│  Goose (in-process llama.cpp)                                   │
│  - Loads the GGUF model file                                    │
│  - Registers RPC workers as ggml backends                       │
│  - Distributes layers across all devices by free memory         │
│  - Sends weights to workers over TCP at startup                 │
│  - Dispatches compute subgraphs each forward pass               │
└───────┬──────────────────┬──────────────────┬───────────────────┘
        │ TCP              │ TCP              │ local
        ▼                  ▼                  ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│  rpc-server   │  │  rpc-server   │  │  Metal GPU    │
│  Machine B    │  │  Machine C    │  │  (built-in)   │
│  :50052       │  │  :50052       │  │               │
└───────────────┘  └───────────────┘  └───────────────┘
```

Workers are **stateless** — they don't need the model file. Goose sends everything over TCP.

## Building rpc-server

The rpc-server workers **must** be built from the same llama.cpp version that goose links against. Mismatched versions silently fail (workers connect but report 0 devices).

Build a matching binary with one command:

```bash
just build-rpc-server
# → ./target/rpc-server
```

This finds the llama.cpp source in Cargo's git cache and builds rpc-server from it. Copy the binary to each worker machine.

## Setup

### 1. Start rpc-server on each worker machine

```bash
# Remote machine (expose over network)
./target/rpc-server -H 0.0.0.0 -p 50052

# Same machine, multiple workers on different ports
./target/rpc-server -d CPU -p 50052 &
./target/rpc-server -d CPU -p 50053 &
```

Verify:
```bash
lsof -i :50052 -i :50053 | grep LISTEN
```

### 2. Configure in goose

**Desktop UI:** Settings → Local Inference → select your model → scroll to **Distributed Inference (RPC)** → enter one `host:port` per line. Reload the model after changing.

**Registry JSON:** Edit `~/.local/share/goose/models/registry.json`:

```json
{
  "settings": {
    "rpc_endpoints": [
      "192.168.1.10:50052",
      "192.168.1.11:50052"
    ]
  }
}
```

That's it. When RPC endpoints are configured, goose automatically:
- Registers each endpoint as a ggml backend device
- Sets `n_gpu_layers=99` to offload all layers
- Lets llama.cpp distribute layers across devices by available memory

### 3. Optional: tensor_split

By default, layers are distributed proportional to each device's free memory. To override:

```json
{
  "settings": {
    "rpc_endpoints": ["127.0.0.1:50052", "127.0.0.1:50053"],
    "tensor_split": "0.4,0.3,0.3"
  }
}
```

Values are proportions per device in order: RPC workers (in registration order), then local GPU. The example above puts 40% on the first worker, 30% on the second, 30% on local Metal.

## Tested Configurations

| Model | Quant | Size | RPC Workers | Result |
|-------|-------|------|-------------|--------|
| GLM-4.7-Flash | Q4_K_M | 17GB | 2× CPU localhost | ✅ Works, layers split across RPC0 + RPC1 + Metal |
| GLM-4.7-Flash | IQ4_NL | 16GB | 2× CPU localhost | ✅ Works (same arch, similar quant) |
| GLM-4.7-Flash | Q8_0 | 30GB | 2× CPU localhost | ❌ Crashes — see below |

## Quick Test (localhost)

Prerequisites: goose built on `local-models-candle` branch, a GLM model downloaded.

```bash
# 1. Build matching rpc-server
just build-rpc-server

# 2. Start two CPU workers
./target/rpc-server -d CPU -p 50052 &
./target/rpc-server -d CPU -p 50053 &

# 3. Verify workers are up
lsof -i :50052 -i :50053 | grep LISTEN
```

Then in the desktop app:
1. Settings → Local Inference
2. Select **GLM-4.7-Flash (IQ4_NL)** — **not Q8_0** (Q8_0 crashes with RPC on this llama.cpp version)
3. Scroll to **Distributed Inference (RPC)**
4. Enter:
   ```
   127.0.0.1:50052
   127.0.0.1:50053
   ```
5. Reload the model (switch away and back, or restart)

In the goosed logs you should see:
```
Registering RPC endpoint: 127.0.0.1:50052
Registering RPC endpoint: 127.0.0.1:50053
load_tensors: layer   0 assigned to device RPC0
...
load_tensors: layer  18 assigned to device RPC1
...
load_tensors: layer  35 assigned to device Metal
```

The `CPU_REPACK` warning (`token_embd.weight cannot be used with preferred buffer type CPU_REPACK, using CPU instead`) is harmless — just an optimized memory layout fallback.

### Stopping workers

```bash
pkill rpc-server
```

## Known Issues

- **Q8_0 + RPC crashes.** The bundled llama.cpp (commit `1051ecd28`, Jan 2025) hits a fatal Metal assertion (`ggml_metal_synchronize`) when using Q8_0 quant with RPC backends. Use Q4_K_M or IQ4_NL instead. This will resolve when the upstream `llama-cpp-rs` crate updates their llama.cpp submodule.
- **Q8_0 without RPC works fine.** The crash is specific to the combination of Q8_0 + RPC + Metal.

## Gotchas

- **Version match matters.** Use `just build-rpc-server` to get a matching binary. Mismatched versions silently fail.
- **First load is slow.** Weights are sent to workers over TCP — expect 30-60s for a 17GB model over localhost, longer over network.
- **Same-machine testing skews the split.** CPU workers report full system RAM, so llama.cpp over-assigns layers to them vs your local GPU. Use `tensor_split` to override (e.g., `"0.1,0.1,0.8"` to keep 80% on Metal). This isn't an issue with real remote machines — each reports its own memory.
- Two rpc-servers on the **same machine** should use different `-d` device flags or different ports.
- Workers can cache weights with `rpc-server -c` for faster restarts.
- The model file is only needed on the machine running goose, not on workers.
- For cross-machine use, ensure the RPC port (default 50052) is reachable — check firewalls.

## Architecture

Goose compiles llama.cpp with `-DGGML_RPC=ON` via a [minimal fork](https://github.com/michaelneale/llama-cpp-rs/tree/rpc) of `llama-cpp-rs` (4 files changed, 23 lines). Before model load, `register_rpc_endpoints()` calls `ggml_backend_rpc_add_server()` then `ggml_backend_register()` for each endpoint — the same pattern as llama-server's `add_rpc_devices()`. No external process is involved; inference runs entirely in-process.
