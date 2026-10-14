import { useState } from "react";
import type { RuntimeSnapshotDto, ProcessSnapshotDto } from "@plc-lab/ipc";
import { TANK_EXAMPLE_SOURCE, tankInterfaceCompatible } from "./tank-example";
function TankRenderer({ snapshot }: { snapshot: ProcessSnapshotDto }) {
  // Only presentation geometry: every state/command/sensor is supplied by Rust.
  const height = (105 * snapshot.levelPermille) / 1000;
  const lowY = 141 - (105 * snapshot.lowThresholdPermille) / 1000;
  const highY = 141 - (105 * snapshot.highThresholdPermille) / 1000;
  const percentage = (snapshot.levelPermille / 10).toFixed(1);
  return (
    <>
      <output
        className="tank-level"
        data-testid="tank-level"
        aria-label="Tank level"
      >
        {percentage} %
      </output>
      <svg
        className="tank-diagram"
        viewBox="0 0 350 170"
        role="img"
        aria-label={`Tank Filling: ${percentage} percent`}
      >
        <defs>
          <clipPath id="tank-vessel">
            <rect x="92" y="36" width="132" height="105" rx="4" />
          </clipPath>
        </defs>
        <path
          d="M 20 44 H 92 M 224 128 H 248 V 146 H 330"
          className="tank-pipe"
        />
        <rect
          x="92"
          y="36"
          width="132"
          height="105"
          rx="4"
          className="tank-vessel"
        />
        <rect
          x="92"
          y={141 - height}
          width="132"
          height={height}
          clipPath="url(#tank-vessel)"
          className="tank-liquid"
          data-testid="tank-fill"
        />
        <path
          d="M 45 34 L 65 54 L 65 34 L 45 54 Z"
          className={snapshot.inletValveOpen ? "device-active" : "device-idle"}
        />
        <circle
          cx="282"
          cy="146"
          r="8"
          className={
            snapshot.drainPumpRunning ? "device-active" : "device-idle"
          }
        />
        <path d="M 279 142 L 286 146 L 279 150 Z" className="pump-arrow" />
        <circle
          cx="230"
          cy={lowY}
          r="4"
          className={snapshot.lowLevel ? "sensor-active" : "sensor-idle"}
        />
        <circle
          cx="230"
          cy={highY}
          r="4"
          className={snapshot.highLevel ? "sensor-active" : "sensor-idle"}
        />
        <text x="10" y="22">
          Inlet {snapshot.inletValveOpen ? "OPEN" : "CLOSED"}
        </text>
        <text x="242" y={lowY + 4}>
          Low {snapshot.lowLevel ? "ACTIVE" : "INACTIVE"}
        </text>
        <text x="242" y={highY + 4}>
          High {snapshot.highLevel ? "ACTIVE" : "INACTIVE"}
        </text>
        <text x="235" y="168">
          Drain {snapshot.drainPumpRunning ? "RUNNING" : "STOPPED"}
        </text>
      </svg>
      <div className="process-readouts">
        <span data-testid="inlet-state">
          Inlet Valve: {snapshot.inletValveOpen ? "OPEN" : "CLOSED"}
        </span>
        <span data-testid="pump-state">
          Drain Pump: {snapshot.drainPumpRunning ? "RUNNING" : "STOPPED"}
        </span>
        <span data-testid="low-state">
          Low Level: {snapshot.lowLevel ? "ACTIVE" : "INACTIVE"}
        </span>
        <span data-testid="high-state">
          High Level: {snapshot.highLevel ? "ACTIVE" : "INACTIVE"}
        </span>
      </div>
    </>
  );
}
export function ProcessPanel({
  source,
  snapshot,
  current,
  canManage,
  onExample,
  onAttach,
  onDetach,
  onReset,
}: {
  source: string;
  snapshot: RuntimeSnapshotDto | null;
  current: boolean;
  canManage: boolean;
  onExample: () => void;
  onAttach: () => Promise<void>;
  onDetach: () => Promise<void>;
  onReset: () => Promise<void>;
}) {
  const [confirm, setConfirm] = useState(false);
  if (snapshot?.process)
    return (
      <div className="tank-scene" data-testid="tank-scene">
        <div className="process-controls">
          <strong>Tank Filling</strong>
          <button
            type="button"
            disabled={!canManage}
            onClick={() => void onReset()}
            title="Empty the tank and restage its sensors. PLC state, count and time stay unchanged."
          >
            Reset Process
          </button>
          <button
            type="button"
            disabled={!canManage}
            onClick={() => void onDetach()}
            title="Remove the plant and return sensor Inputs to manual control."
          >
            Detach Process
          </button>
        </div>
        {!current && (
          <p className="process-stale">
            Previous loaded source · STALE · process execution disabled
          </p>
        )}
        <TankRenderer snapshot={snapshot.process} />
        <p className="process-help">
          Sensors feed the next scan. Actuators show the last applied interval.
          Reset Runtime restarts the PLC and preserves this plant.
        </p>
      </div>
    );
  const compatible = tankInterfaceCompatible(snapshot);
  const canAttach =
    current && canManage && snapshot?.state === "READY" && compatible;
  return (
    <div className="process-empty" data-testid="process-empty">
      <h3>No process scene attached</h3>
      <strong>Tank Filling Demo</strong>
      <div className="process-controls">
        <button
          type="button"
          className="compile-button"
          onClick={() =>
            source === TANK_EXAMPLE_SOURCE ? onExample() : setConfirm(true)
          }
        >
          Use Example
        </button>
        <button
          type="button"
          className="compile-button"
          disabled={!canAttach}
          onClick={() => void onAttach()}
        >
          Attach Tank Scene
        </button>
      </div>
      <p>
        Follow the inlet valve filling a tank until High feedback closes it on
        the next scan. The drain pump empties it when commanded.
      </p>
      <details>
        <summary>Required PLC interface</summary>
        <code>
          VAR_INPUT: LevelLow BOOL, LevelHigh BOOL, LevelPercent REAL
          <br />
          VAR_OUTPUT: InletValve BOOL, DrainPump BOOL
        </code>
      </details>
      <p className="process-help">
        Use Example → Compile → Load Program → Attach Tank Scene → Single Scan
      </p>
      {!canAttach && (
        <p className="process-help">
          {!snapshot
            ? "Compile and load a compatible PLC program first."
            : !current
              ? "Source changed or association unavailable. Compile and load again."
              : !compatible
                ? "Loaded program does not provide the required Tank interface."
                : snapshot.state !== "READY"
                  ? "Reset the faulted runtime first."
                  : "Wait for the current request to finish."}
        </p>
      )}
      {confirm && (
        <div
          className="source-confirmation"
          role="alertdialog"
          aria-label="Replace Structured Text source"
        >
          <p>
            Replace the current editor text with the Tank Filling example? This
            changes source only; compile, load and attach remain explicit.
          </p>
          <button
            type="button"
            autoFocus
            onClick={() => {
              setConfirm(false);
              onExample();
            }}
          >
            Replace Source
          </button>
          <button type="button" onClick={() => setConfirm(false)}>
            Cancel
          </button>
        </div>
      )}
    </div>
  );
}
