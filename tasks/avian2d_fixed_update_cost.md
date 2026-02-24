# Avian2d Fixed Update Cost

## Problem

`RunFixedMainLoop` takes ~6ms per frame. This schedule runs Avian2d's full physics pipeline in `FixedPostUpdate` — broadphase, narrowphase, constraint solving, collision detection, mass property updates, collider transforms — every fixed timestep tick.

We only use Avian2d for static wall colliders and sensor triggers (pen gate, collectible pickups). There is no continuous physics, no rigid body movement, no forces. Movement is purely grid-based. 6ms per frame on idle physics is waste.

## Discovery

Found via the Bevy tracing bridge (`src/tracing_bridge.rs`) which surfaces per-schedule timing. Without schedule-level spans, this time was invisible — it appeared as "empty space" between our instrumented systems.

## Options

### 1. Increase fixed timestep interval

Configure `Time<Fixed>` to tick less frequently (e.g. 30Hz instead of the default 64Hz). Fewer ticks per frame means less total physics work. Doesn't eliminate the cost, just reduces it proportionally.

### 2. Move Avian to variable timestep

Run physics in `Update` instead of `FixedPostUpdate` via `PhysicsPlugins::new(Update)`. Since we don't need deterministic physics (no networked simulation, no replays), fixed timestep buys us nothing. This eliminates `RunFixedMainLoop` cost entirely and runs physics once per frame.

### 3. Strip down Avian plugin set

Replace `PhysicsPlugins::default()` with only the sub-plugins we need (collision detection, spatial queries). Skip the dynamics solver, integrator, and sleeping systems entirely. Most targeted fix but requires understanding Avian's internal plugin decomposition.

### 4. Replace Avian with manual grid collision

Remove Avian entirely. Wall collision is already handled by the grid walkability check in `movement_validation`. Sensor triggers (pen gate, collectibles) could use simple grid-coordinate overlap checks instead of physics sensors. Eliminates the dependency and all associated overhead.

## Recommendation

Option 2 (variable timestep) is the lowest-effort fix with the biggest payoff. One-line change to plugin registration. Option 4 is the cleanest long-term but requires migrating sensor trigger logic.

## Status

Not started — logged for future optimization.
