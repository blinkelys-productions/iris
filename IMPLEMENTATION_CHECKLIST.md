# Iris implementation checklist

## What we are building

Iris is an open-source desktop lighting console: a reliable Rust lighting engine and shared show model, operated through a Vue/Tauri interface, with fixture patching, programming and playback, network output, integrations, and a 3D visualizer. The engine must remain deterministic, hardware- and protocol-independent, and able to run without the UI or visualizer.

This checklist turns the product specification in [whatarewedoing.md](./whatarewedoing.md) into implementation milestones. Check an item only when the behavior is implemented and its relevant tests pass.

## Current focus: showfiles, saving, loading and quicksaves

Before wiring the UI to the backend, define how shows are stored. Work happens in `src-tauri/src/domain/persistence.rs`.

- [x] Define the show file envelope (format marker, version, app version, timestamp) with encode/decode and validation.
- [x] Define quicksave models and a bounded history that evicts only automatic entries.
- [x] Define the on-disk library layout (show, quicksaves, backups) and persistence errors.
- [x] Read and write show files atomically (temp file, then rename) and list saved shows (`library.rs`).
- [x] Create, save, save-as, load and delete shows in the library; reject ids that could escape it.
- [x] Quicksave create, list, restore and delete, with automatic quicksaves pruned on disk.
- [x] Hold the current show in the engine (`session.rs`): dirty tracking, validated edits with rollback, save/save-as, manual quicksaves, and an automatic quicksave policy driven by a caller-supplied clock.
- [x] Backups: the previous saved file is kept in `backups/` on every save (bounded), and backups can be listed, restored and deleted.
- [x] Crash and corruption recovery: a corrupt show file is moved aside and replaced by the newest valid backup or quicksave; opening a show reports when this happened.
- [x] Run the automatic quicksave timer in the app (every 15 s, policy in the session) and store shows under the app data directory.
- [x] Tauri commands for shows, quicksaves and backups (`service.rs` facade plus thin commands in `lib.rs`).
- [x] Format migrations between show format versions (`MIGRATIONS` table in `persistence.rs`; empty while the format is at version 1).
- [x] Wire the Menu's Show, Backup and Quick Save tiles to the commands (`ShowDialog.vue`, `stores/shows.ts`); Cmd/Ctrl+S saves. Only works inside the Tauri app.
- [x] Persist the workspace layout (views and windows) and the patch in the show file, so loading restores them.
- [x] Connect the patch UI to the Rust patch model via Tauri commands (`get_patch`, `save_fixtures`, `remove_fixtures`); edits are validated by the backend and stored with the open show.

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
- [x] Replace the placeholder fixture library with profiles imported through the Rust GDTF importer (stored in `<app data>/profiles`; the placeholders remain only for the browser preview).
- [x] Patch window: Import GDTF, Import MVR, Export MVR and Export PDF via Tauri commands and native file dialogs.
- [ ] Run the real Tauri app end to end (`cargo tauri dev`): show create/save/reopen, patch edits, imports and exports. Not yet done.

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
