# Iris implementation checklist

## What we are building

Iris is an open-source desktop lighting console: a reliable Rust lighting engine and shared show model, operated through a Vue/Tauri interface, with fixture patching, programming and playback, network output, integrations, and a 3D visualizer. The engine must remain deterministic, hardware- and protocol-independent, and able to run without the UI or visualizer.

This checklist turns the product specification in [whatarewedoing.md](./whatarewedoing.md) into implementation milestones. Check an item only when the behavior is implemented and its relevant tests pass.

## 1. Core model and patching

- [x] Add Rust fixture-definition, mode, fixture-instance, and patch types.
- [x] Validate fixture and mode IDs, fixture numbers, profile references, universes, and DMX address ranges.
- [x] Detect fixture address collisions and test patch-validation boundaries.
- [x] Define the shared show model and keep persistent show data separate from runtime state.
- [x] Define stable IDs, typed commands, events, and actionable domain errors.
- [x] Support fixture create, edit, move, replace, and delete operations while preserving identity where appropriate.
- [x] Import single-fixture, single-break GDTF profiles into the patch model and preserve the original GDTF archive unchanged for export.
- [x] Import/export MVR 1.4-1.6 fixture patches with embedded GDTF archives; report when scene data is not represented.
- [x] Preserve and round-trip finite 4×4 fixture transform matrices in MVR exports.
- [ ] Preserve and edit complete GDTF/MVR data, including multi-break patching, geometry, and other scene objects.
- [x] Export a printable patch list as PDF.
- [x] Build the UI shell: grandMA3-style workspace (icon rail, dotted workspace, 8-view strip, command bar, programmer panel placeholder) and main menu.
- [x] Add a patch window with add/edit/delete and live conflict reporting (local state only).
- [ ] Connect the patch UI to the Rust patch model via Tauri commands (replacing local state and the placeholder fixture library).

## 2. Programming data and attributes

- [ ] Define typed lighting attributes, units, discrete values, and DMX channel mappings.
- [ ] Implement fixture selection and programmer state independently of playback.
- [ ] Add fixture groups and partial, compatible presets.
- [ ] Implement deterministic attribute resolution and report each attribute's current source.
- [ ] Build UI workflows for selecting fixtures and editing attributes.

## 3. Cues, playback, and effects

- [ ] Define sequences, cues, tracking, timing, and recipe semantics.
- [ ] Implement multiple concurrent playbacks and document priority, HTP/LTP, and release behavior.
- [ ] Keep playback timing in the engine, independent of UI frame rate.
- [ ] Implement deterministic, timestamp-driven phasers and effects.
- [ ] Add automated tests for cue timing, tracking, playback resolution, and effect behavior.
- [ ] Build UI workflows for programming cues and controlling playbacks.

## 4. Network output and external control

- [ ] Add network-interface discovery, explicit configuration, and diagnostics.
- [ ] Implement and test sACN output, including frame encoding and output lifecycle.
- [ ] Implement Art-Net as a separate protocol adapter.
- [ ] Add OSC commands and feedback through the shared command system.
- [ ] Add MIDI and hardware control surfaces through the same command architecture.
- [ ] Test protocol packets, invalid configurations, and output fault reporting.

## 5. Visualization and pixel mapping

- [ ] Define scene, fixture-placement, and renderer-facing state without making the renderer authoritative.
- [ ] Establish a testable rendering module and a native Vulkan surface strategy.
- [ ] Render fixture movement, color, intensity, and beams from resolved lighting state.
- [ ] Add stage/model import and scene navigation.
- [ ] Add bitmap assets, pixel mapping, and deterministic mapping tests.
- [ ] Ensure renderer and bitmap workloads cannot block lighting output.

## 6. Operations and future expansion

- [ ] Add show persistence, versioning, and recovery behavior.
- [ ] Add output health, performance diagnostics, and clear operator-facing errors.
- [ ] Document supported formats, protocols, and user workflows.
- [ ] Define multi-user identity, permissions, ownership, and conflict behavior.
- [ ] Implement multi-user synchronization after the single-console workflow is reliable.
- [ ] Keep builds reproducible and maintain automated tests for each milestone.
