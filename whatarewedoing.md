# Iris — Product Vision, Engineering Specification & AI-Agent Roadmap

**Project:** Iris  
**Category:** Open-source professional lighting control platform  
**Application:** Native desktop console with custom hardware support  
**Frontend:** Vue 3 + TypeScript  
**Application framework:** Tauri 2  
**Core language:** Rust  
**Graphics API:** Vulkan  
**Current application version:** 0.1.0

## 1. Product vision

Iris is an open-source lighting control platform designed to compete with and ultimately surpass established professional lighting consoles, including grandMA3.

The project aims to combine powerful lighting programming, a flexible playback engine, advanced effects, a modern 3D visualizer, open integrations, and purpose-built hardware in one cohesive system.

Iris must not be a collection of disconnected features. Every subsystem should share a consistent show model, attribute model, timing system, fixture definition system, and command architecture.

The application should support both traditional live lighting operation and advanced programming workflows. Its architecture must accommodate small rigs, large productions, complex fixture configurations, external control systems, and future distributed operation.

### Core principles

1. **Show-first architecture:** All programming and playback systems operate on a shared, well-defined show model.
2. **Deterministic lighting engine:** Identical inputs, state, and timing should produce identical resolved lighting output, within explicitly defined numerical tolerances.
3. **Hardware independence:** The software must not depend on a specific control surface or console hardware to operate.
4. **Protocol independence:** Lighting logic must not depend on sACN, Art-Net, or any particular transport.
5. **Extensibility:** Fixture profiles, effects, control surfaces, protocols, and future rendering capabilities should be extendable without rewriting the core.
6. **Operator efficiency:** Common programming and playback operations must be fast, predictable, and usable under live-show conditions.
7. **Open development:** Prefer documented interfaces, reproducible builds, automated tests, and clearly licensed dependencies.
8. **Fault isolation:** A failure in the UI or visualizer should not unnecessarily interrupt the lighting engine or network output.
9. **Explicit state:** Commands, persistent show data, runtime state, and resolved output must have clearly defined ownership.
10. **Incremental delivery:** Every milestone should leave Iris in a buildable, testable state.

## 2. Technology and repository architecture

### Repository layout

```text
iris/
├── ui/                         # Vue 3 + TypeScript
│   └── src/
│       ├── components/
│       ├── views/
│       ├── stores/
│       ├── composables/
│       ├── services/
│       └── types/
│
├── src-tauri/                  # Tauri application + Rust
│   ├── src/
│   │   ├── commands/
│   │   ├── application/
│   │   ├── domain/
│   │   ├── engine/
│   │   ├── fixtures/
│   │   ├── programming/
│   │   ├── playback/
│   │   ├── protocols/
│   │   ├── hardware/
│   │   ├── renderer/
│   │   ├── persistence/
│   │   └── main.rs / lib.rs
│   ├── tests/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── docs/
│   ├── architecture/
│   ├── specifications/
│   ├── protocols/
│   ├── hardware/
│   ├── decisions/
│   └── roadmap/
│
├── fixtures/                   # Versioned fixture definitions
├── examples/                   # Example shows and test rigs
├── scripts/
├── techstack.md
├── CONTRIBUTING.md
├── LICENSE
└── README.md
```

This is the intended logical structure, not a requirement to create every directory immediately. Follow existing repository conventions where they are already established, and introduce modules as the relevant milestones begin.

### Responsibility boundaries

**Vue frontend**

Owns the operator interface, panel layout, user interaction, visual feedback, and frontend-only presentation state.

**Tauri application layer**

Provides the desktop shell, permissions, command registration, window management, and communication between the frontend and Rust services.

**Rust domain layer**

Defines fixtures, attributes, groups, presets, sequences, cues, playbacks, patching, and show data. This layer should remain testable without Tauri.

**Rust lighting engine**

Owns live runtime state, timing, attribute resolution, HTP/LTP behavior, effects evaluation, and final DMX frame generation.

**Protocol adapters**

Translate between the internal output model and external protocols. They must not implement separate lighting logic.

**Renderer**

Consumes shared show and resolved fixture state. It visualizes fixture movement, colors, beams, geometry, and atmospheric effects without becoming the authority for lighting output.

**Hardware layer**

Abstracts physical control surfaces, encoders, faders, displays, buttons, and other supported hardware.

## 3. Core domain model

Before implementing complex features, establish the foundational data types and their relationships.

### 3.1 Show

A show contains:

- Fixture definitions and patch assignments.
- Fixture groups.
- Presets and palettes.
- Sequences, cues, and recipes.
- Playback configuration.
- Effects and phaser definitions.
- Network configuration.
- Stage and 3D visualization data.
- User preferences and show metadata.

Persistent show data must be separate from transient runtime state.

### 3.2 Fixture

A fixture instance should reference a fixture definition and contain:

- Stable fixture ID.
- Fixture type and manufacturer metadata.
- Fixture name, number, and user-defined tags.
- Universe and DMX start address.
- Selected DMX mode.
- Instance-level configuration.
- Optional physical position and orientation.
- Optional visualization model and geometry configuration.

The fixture definition describes the capabilities of a fixture. The fixture instance describes a particular physical or virtual fixture in a show. They must be able to fetch from GDTF, mvr, export to mvr and also a simple patch list to a .pdf

### 3.3 Attribute

Represent lighting parameters as typed attributes rather than treating all DMX channels as interchangeable integers.

The model must support:

- Intensity.
- Color, including RGB, RGBW, CMY, and other supported mixing systems.
- Position, including pan and tilt.
- Zoom, focus, iris, and beam parameters.
- Shutter and strobe.
- Gobo selection and rotation.
- Prism and effect parameters.
- Discrete parameters and enumerated values.
- Fine/coarse DMX parameters.
- Multi-channel and virtual attributes.
- Fixture-specific parameters.

Attribute definitions must distinguish normalized values, physical units, discrete values, and raw DMX representation.

### 3.4 Runtime state

Runtime state includes:

- Current programmer values.
- Active playbacks and their levels.
- Running sequences and current cue positions.
- Active effects and their phases.
- Attribute ownership and priority.
- Current resolved fixture output.
- Network output health.
- Active control inputs.

The engine must define how values from different sources interact, including the programmer, playbacks, effects, and external controllers.

### 3.5 Commands and events

Use explicit, typed commands and events.

Example commands:

- `PatchFixture`
- `SetAttribute`
- `StorePreset`
- `StoreCue`
- `GoSequence`
- `SetPlaybackLevel`
- `ReleasePlayback`
- `SetUniverseOutput`
- `CaptureProgrammerState`

Example events:

- `FixturePatched`
- `AttributeChanged`
- `CueStarted`
- `CueCompleted`
- `PlaybackLevelChanged`
- `UniverseOutputUpdated`
- `NetworkOutputFaulted`

These names illustrate the intended model. Final names and payload schemas must be defined in the relevant specifications.

Commands request changes. Events report changes. The engine remains the authority on whether a command succeeds.

## 4. Feature specifications

### 4.1 Patch system

**Objective:** Provide a complete fixture patching and configuration workflow.

Requirements:

- Import and manage fixture definitions.
- Create, duplicate, edit, move, and delete fixture instances.
- Assign fixture IDs, names, numbers, types, modes, universes, and addresses.
- Validate DMX footprint and universe boundaries.
- Detect address collisions and incompatible assignments.
- Support fixture selection by ID, number, type, group, and range.
- Support bulk patching and fixture replacement.
- Preserve fixture identity when appropriate during replacement.
- Support import/export of fixture definitions.
- Provide a clear patch overview and conflict reporting.
- Validate fixture definitions before they can be used.
- Support fixture profile versioning.

Acceptance criteria:

- Invalid addresses cannot silently enter the show model.
- Overlapping assignments are detected and clearly reported.
- Changing a fixture mode recalculates its channel footprint.
- Patch changes update the engine and visualizer through the shared model.
- Unit tests cover address boundaries, collision detection, mode changes, and fixture replacement.

### 4.2 Attribute editing and programmer

**Objective:** Implement a central programming environment for editing fixture attributes.

Requirements:

- Select individual fixtures or multiple fixtures.
- Edit attributes through numerical entry, encoders, sliders, and suitable controls.
- Support normalized and physical values where applicable.
- Support discrete parameter selection.
- Support relative and absolute edits.
- Support attribute selection and filtering.
- Display current values and their source.
- Support copying and applying attribute values.
- Maintain a programmer state separate from playback output.
- Support clearing individual attributes, selected fixtures, and the complete programmer.
- Provide configurable parameter defaults and encoder behavior.
- Support fixture-specific attribute capabilities.

Acceptance criteria:

- Editing one attribute does not unintentionally overwrite unrelated attributes.
- Multi-fixture operations behave predictably when fixtures have different capabilities.
- Programmer values are independently testable from the UI.
- Invalid values are rejected or clamped according to explicit attribute rules.
- The UI can inspect the resolved output and determine which source currently owns an attribute.

### 4.3 Network system

**Objective:** Centralize network configuration and diagnostics.

Requirements:

- Enumerate available network interfaces.
- Allow explicit interface selection.
- Display interface addresses and relevant network information.
- Configure protocol-specific interfaces and settings.
- Provide universe routing and output configuration.
- Monitor output health, packet rates, and errors.
- Detect invalid or conflicting configurations.
- Provide clear diagnostics for multicast and broadcast configuration.
- Support multiple interfaces where the operating system and network design permit.
- Keep network discovery and monitoring separate from the critical output path.

Acceptance criteria:

- Interface selection is explicit and inspectable.
- Invalid configurations generate actionable errors.
- The engine can report output health without blocking lighting calculations.
- Network diagnostics distinguish configuration errors from transmission failures.

### 4.4 sACN and Art-Net output

**Objective:** Deliver reliable DMX-over-network output.

Requirements:

- Implement sACN output first.
- Add Art-Net output through a separate adapter.
- Support configurable universe mapping.
- Support the required protocol priorities, addressing, and synchronization features according to each protocol specification.
- Handle packet timing, sequence fields, and protocol-specific frame requirements.
- Support unicast and multicast where appropriate.
- Expose output enable/disable controls and clear fault states.
- Define startup, shutdown, and transmission-failure behavior.
- Keep frame generation independent of UI frame rate.
- Provide protocol conformance tests and packet-level tests.
- Allow test output to be captured and inspected without physical fixtures.

Acceptance criteria:

- A deterministic test input produces the expected DMX frame.
- Protocol adapters correctly encode and transmit the frame.
- Output loops continue operating independently of UI rendering.
- Invalid or disconnected output configurations do not crash the application.
- Tests verify frame length, universe mapping, channel values, and protocol-specific fields.

### 4.5 Presets and groups

**Objective:** Provide reusable programming data and efficient fixture selection.

Groups:

- Store fixture membership.
- Support named and numbered groups.
- Support updating membership.
- Support group selection and deselection.
- Define how group membership behaves when fixtures are removed or replaced.
- Support group-based programming operations.

Presets:

- Store selected attributes independently of cue execution.
- Support partial presets.
- Support absolute and relative values where appropriate.
- Support fixture-type compatibility and parameter mapping.
- Support updating, replacing, renaming, and deleting presets.
- Support applying presets to compatible fixtures.
- Define how presets behave when fixture definitions change.
- Support clear indication of referenced presets and dependencies.

Acceptance criteria:

- Applying a preset affects only the attributes it contains.
- Incompatible attributes are handled explicitly.
- Group membership remains consistent with patch changes.
- Preset references do not become silently invalid after edits.

### 4.6 Sequences, cues, and recipes

**Objective:** Implement reusable and flexible show programming.

Sequences:

- Contain an ordered collection of cues.
- Support cue selection and navigation.
- Support forward and backward execution.
- Support sequence configuration and playback behavior.
- Support insertion, deletion, duplication, and reordering of cues.

Cues:

- Store attribute changes, timing information, and relevant programming metadata.
- Support fade-in and fade-out timing.
- Support delays and per-attribute timing where applicable.
- Support cue labels and notes.
- Support tracking and cue-state evaluation.
- Support cue updates without corrupting unrelated cues.
- Define behavior for missing fixtures and changed fixture capabilities.

Recipes:

- Reference reusable programming data, such as presets, groups, or other supported sources.
- Define how values are resolved against target fixtures.
- Support fixture-type compatibility and attribute mapping.
- Define explicit update and re-evaluation behavior.
- Detect broken references and invalid recipe dependencies.
- Keep recipe evaluation deterministic and testable.

The exact semantics of recipes must be specified before implementation. Do not assume that a recipe is simply a stored collection of DMX values.

Acceptance criteria:

- A sequence can execute cues in a defined order.
- Cue timing is controlled by the engine, not by frontend timers.
- Cue evaluation produces deterministic attribute changes.
- Tracking, timing, and recipe resolution have dedicated tests.
- Editing a referenced preset or recipe has documented, predictable consequences.

### 4.7 Playback system

**Objective:** Provide the runtime control layer for executing programmed content.

Requirements:

- Support multiple concurrent playbacks.
- Provide configurable playback levels.
- Support go, back, pause, resume, release, and stop operations where applicable.
- Support manual and timed transitions.
- Support playback priorities and attribute ownership.
- Define HTP and LTP resolution rules.
- Support configurable release behavior.
- Support playback status and progress reporting.
- Support command sources from the UI, OSC, MIDI, and hardware.
- Ensure that external control and the frontend use the same playback commands.
- Provide clear visual indication of active playbacks and conflicts.

Acceptance criteria:

- Multiple playbacks resolve into one deterministic final output.
- Releasing a playback follows documented ownership and fade behavior.
- A playback operation does not require a frontend window to remain active.
- Playback timing is independent of UI frame rate.
- Priority and attribute-merging behavior is covered by automated tests.

### 4.8 Phasers and effects

**Objective:** Create a flexible effects system capable of driving fixture attributes over time.

Requirements:

- Define reusable phaser and effect data.
- Support steps, phases, and parameterized waveforms.
- Support speed, rate, phase offset, width, and amplitude where relevant.
- Support fixture offsets and group-based distribution.
- Support synchronized and free-running effects.
- Support effect direction and phase relationships.
- Support multiple simultaneous effects.
- Define how effects interact with programmer values and playback ownership.
- Support configurable timing and transition behavior.
- Provide effect editing and live preview.
- Keep effect evaluation independent of rendering.
- Support deterministic time-based testing.

Acceptance criteria:

- An effect can be evaluated at an explicit timestamp without depending on wall-clock timing.
- Multiple fixture offsets produce the expected phase relationships.
- Effect output merges into the lighting engine through defined attribute-resolution rules.
- Effects remain functional without the 3D viewer running.
- Tests cover phase, rate, boundaries, synchronization, and release behavior.

### 4.9 Bitmaps

**Objective:** Support pixel-mapping workflows and bitmap-driven lighting effects.

Requirements:

- Import and manage bitmap assets.
- Define mapping surfaces and pixel arrangements.
- Map pixels to fixtures or supported fixture regions.
- Support pixel ordering, orientation, and mapping transformations.
- Support image scaling, cropping, and positioning.
- Support bitmap playback and animation where applicable.
- Define how image colors and brightness map to fixture attributes.
- Support frame-rate control and interpolation where appropriate.
- Optimize large pixel maps.
- Separate bitmap processing from the main output loop.
- Validate memory and performance limits.

Acceptance criteria:

- Identical bitmap input and mapping configuration produce identical mapped values.
- Mapping changes can be previewed without transmitting output.
- Large bitmap workloads cannot indefinitely block lighting output.
- Tests cover pixel order, orientation, color conversion, and mapping boundaries.

### 4.10 Custom 3D viewer

**Objective:** Develop a custom Vulkan-based lighting visualization engine.

Requirements:

- Render fixtures, trusses, stages, scenery, and screens.
- Import supported 3D models.
- Represent fixture position, orientation, geometry, and mechanical movement.
- Visualize pan, tilt, color, intensity, zoom, and supported fixture parameters.
- Render beam geometry and visible light beams.
- Support atmospheric scattering and volumetric effects as the renderer matures.
- Support camera navigation and scene inspection.
- Support stage construction and fixture placement.
- Display the current resolved fixture state.
- Keep rendering state separate from authoritative lighting state.
- Support performance diagnostics and configurable quality settings.
- Define a rendering abstraction that allows future experimentation without rewriting the lighting engine.

Acceptance criteria:

- The renderer accurately reflects the shared fixture state.
- Renderer failure does not corrupt show data or playback state.
- Heavy rendering workloads do not block the lighting output loop.
- Fixture transforms and attribute mappings are tested independently of rendering.
- GPU resource creation and destruction follow explicit ownership and lifecycle rules.

Implementation note: Vulkan integration with a Tauri application requires deliberate native window and surface management. Begin with an independently testable renderer module and establish a rendering-surface strategy before committing to deep UI integration.

### 4.11 OSC integration

**Objective:** Expose a consistent external control and feedback interface.

Requirements:

- Define a versioned OSC address namespace.
- Support playback, cue, sequence, attribute, and relevant system commands.
- Support feedback messages for state changes.
- Support configurable listen and destination addresses.
- Support input validation and rate limiting where necessary.
- Define authentication and network trust boundaries for remote control.
- Support external controller mappings.
- Provide a discoverable protocol reference.
- Ensure OSC commands enter through the same command system as UI actions.
- Define behavior for invalid messages, missing arguments, and unsupported commands.

Acceptance criteria:

- Every supported OSC command maps to a documented application command.
- Feedback accurately represents authoritative engine state.
- Invalid messages do not crash the application.
- Protocol tests cover address parsing, payload types, command execution, and feedback.

### 4.12 Multi-user system

**Objective:** Allow multiple operators or clients to interact with a shared show.

Requirements:

- Define user identity and session management.
- Define permissions and access levels.
- Define ownership of programmer state and playback controls.
- Support shared show data and authoritative runtime state.
- Define conflict handling for simultaneous edits.
- Support synchronization of relevant show and runtime changes.
- Define reconnect, timeout, and disconnected-client behavior.
- Record relevant operations for diagnostics and auditability.
- Support local-only operation when no remote clients are connected.
- Keep transport and authentication separate from show-domain logic.

Acceptance criteria:

- Simultaneous operations have deterministic, documented outcomes.
- A disconnected client cannot retain effective control indefinitely.
- Shared show state remains consistent across authorized clients.
- Local lighting output can continue if a remote client disconnects.
- Authentication, authorization, synchronization, and conflict handling are independently tested.

Multi-user support is a major architectural milestone. Design the command and state model to accommodate it early, but defer the full distributed implementation until the single-console