## Context

Current Selium designs use host-managed shared-memory handles and queue references as a workaround for Wasmtime's limitations around shared guest-visible memory. `wasmtiny` exists in part to remove that constraint and make zero-copy inter-guest data exchange a runtime primitive rather than a hostcall pattern.

## Goals / Non-Goals

**Goals:**
- Define runtime-owned shared memory regions that are distinct from per-instance private memory.
- Allow multiple instances to attach the same region safely.
- Provide zero-copy visibility semantics for mapped shared bytes.
- Make attachment and detachment explicit so higher layers can manage lifetime safely.

**Non-Goals:**
- Defining Selium queue semantics.
- Solving full cluster replication or distributed shared memory.
- Coupling shared memory design to one specific host capability API.

## Decisions

### 1. Shared memory is a runtime object, not an ad hoc host import
The runtime will manage shared regions as first-class objects with stable identity and explicit attachment rules, rather than requiring callers to smuggle shared state through normal imported memories.

### 2. Shared regions and private linear memory remain distinct concepts
Private guest memory keeps its existing isolation semantics. Shared regions are separately allocated runtime objects that can be attached where the runtime allows, which keeps ownership and migration semantics clearer.

### 3. Attach/detach is explicit
Instances must opt into a shared region, and the runtime must know when a mapping is removed. This supports lifetime checks, migration checks, and future snapshot rules.

### 4. Coherence is part of the runtime contract
Writes performed through one attached mapping must become visible to other attached mappings according to the runtime's documented memory model. The runtime must not silently route shared access through hidden copy buffers.

## Risks / Trade-offs

- [Alias complexity in JIT] -> Start with a correctness-first interpreter path and add JIT alias handling behind the same contract.
- [Over-committing the mapping model too early] -> Keep the public API focused on regions, attachments, and visibility guarantees rather than one specific address-space layout.
- [Snapshot interaction] -> Make attachment metadata explicit so snapshot/restore can define supported and unsupported cases clearly.
- [Use-after-detach or stale handles] -> Treat region and mapping handles as validated runtime objects with explicit failure paths.
