/** Contract-valid fixtures for UI tests. Every fixture passes the strict validators. */
import type {
  LibraryDto,
  LiveFrameDto,
  ProjectDto,
  WorkspaceDto,
} from "@plc-lab/ipc";

export function frame(overrides: Partial<LiveFrameDto> = {}): LiveFrameDto {
  return {
    workspaceId: "1",
    worldTimeUs: "1200000",
    cycle: "12",
    simulation: {
      state: "PAUSED",
      speedPercent: 100,
      runId: null,
      cycleUs: "100000",
      failure: null,
    },
    plc: {
      mode: "RUN",
      switch: "RUN",
      programName: "Main",
      programRevision: "2",
      segment: 1,
      scans: "12",
      taskTimeUs: "1200000",
      fault: null,
      forces: 0,
      leds: [{ name: "RUN/STOP", state: "GREEN" }],
    },
    components: [
      {
        id: "start",
        quantities: [
          { name: "pressed", value: { type: "BOOL", value: false } },
        ],
      },
      {
        id: "motor",
        quantities: [
          { name: "command", value: { type: "BOOL", value: true } },
          { name: "running", value: { type: "BOOL", value: true } },
          { name: "speedPermille", value: { type: "COUNT", value: 800 } },
          { name: "currentMa", value: { type: "COUNT", value: 4200 } },
          { name: "tripped", value: { type: "BOOL", value: false } },
        ],
      },
    ],
    watch: [],
    causeEffect: {
      cycle: "11",
      startUs: "1100000",
      endUs: "1200000",
      segment: 1,
      outcome: "SCANNED",
      saw: [
        { signal: 10, value: { type: "BOOL", value: true }, changed: true },
      ],
      decided: [
        { signal: 11, value: { type: "BOOL", value: true }, changed: true },
      ],
      applied: [
        { signal: 21, value: { type: "BOOL", value: true }, changed: true },
      ],
      process: [
        {
          signal: 20,
          before: { type: "BOOL", value: false },
          after: { type: "BOOL", value: true },
        },
      ],
      next: [
        { signal: 12, value: { type: "BOOL", value: true }, changed: true },
      ],
      fault: null,
    },
    alarms: [],
    activeDiagnostics: 0,
    latestEvent: "4",
    trace: {
      retainedCycles: "12",
      droppedCycles: "0",
      usedBytes: "4096",
      budgetBytes: "16777216",
    },
    signalsRevision: 2,
    ...overrides,
  };
}

export const station: ProjectDto["station"] = {
  name: "PLC_1",
  cpu: "cpu.compact",
  modules: [
    {
      position: { rack: 0, slot: 1 },
      module: "di8.24vdc.sink",
      startAddress: 0,
      channels: [],
    },
  ],
  ioMap: [{ channel: { rack: 0, slot: 1, channel: 0 }, tag: "StartPB" }],
  network: { interface: null, modbusTcp: [] },
};

export function project(overrides: Partial<ProjectDto> = {}): ProjectDto {
  return {
    id: "p-1",
    name: "Motor start/stop",
    kind: "exercise",
    exercise: "ex.motor-start-stop",
    platform: "generic.iec",
    scene: "scene.motor-start-stop",
    station,
    programs: [
      {
        id: "main",
        name: "Main",
        language: "ST",
        source: "PROGRAM Main\nEND_PROGRAM\n",
      },
    ],
    watch: [],
    simulation: { cycleMs: 100, speedPercent: 100 },
    createdAt: "2026-10-27T10:00:00Z",
    updatedAt: "2026-10-27T10:00:00Z",
    ...overrides,
  };
}

export function workspace(overrides: Partial<WorkspaceDto> = {}): WorkspaceDto {
  return {
    workspaceId: "1",
    projectId: "p-1",
    scene: {
      id: "scene.motor-start-stop",
      title: "Motor start/stop station",
      summary: "A conveyor motor with start and stop buttons.",
      components: [
        {
          id: "start",
          label: "Start S1",
          kind: "pushButton",
          color: "green",
          positions: [],
          operable: true,
          config: [{ key: "contact", value: "normallyOpen" }],
        },
        {
          id: "motor",
          label: "Conveyor motor M1",
          kind: "motorStarter",
          color: null,
          positions: [],
          operable: false,
          config: [],
        },
      ],
      layout: [
        { component: "start", x: 120, y: 420, width: null, height: null },
        { component: "motor", x: 620, y: 260, width: 220, height: 160 },
      ],
      couplings: [],
      interface: [
        {
          tag: "StartPB",
          dataType: "BOOL",
          direction: "input",
          description: "Start button",
        },
      ],
      hasNetwork: false,
    },
    platform: {
      id: "generic.iec",
      displayName: "Generic IEC 61131-3 PLC",
      vendor: "generic",
      family: "iec-61131",
      tier: "GENERIC",
      fidelity: {
        level: "APPROXIMATED_BEHAVIORAL",
        modeled: [],
        approximated: [],
        omitted: [],
      },
      addressing: "IEC direct addresses (%IX0.0)",
      analog: "0..10000",
    },
    signals: [
      {
        id: 10,
        path: "plc.StartPB",
        label: "StartPB",
        kind: "PLC_INPUT",
        valueType: "BOOL",
        unit: null,
        active: true,
      },
      {
        id: 11,
        path: "plc.MotorContactor",
        label: "MotorContactor",
        kind: "PLC_OUTPUT",
        valueType: "BOOL",
        unit: null,
        active: true,
      },
      {
        id: 12,
        path: "plc.MotorFeedback",
        label: "MotorFeedback",
        kind: "PLC_INPUT",
        valueType: "BOOL",
        unit: null,
        active: true,
      },
      {
        id: 21,
        path: "process.motor.command",
        label: "Conveyor motor M1 command",
        kind: "PHYSICAL",
        valueType: "BOOL",
        unit: null,
        active: true,
      },
      {
        id: 20,
        path: "process.motor.running",
        label: "Conveyor motor M1 running",
        kind: "PHYSICAL",
        valueType: "BOOL",
        unit: null,
        active: true,
      },
    ],
    exercise: {
      id: "ex.motor-start-stop",
      revision: 1,
      title: "Motor start/stop with seal-in",
      kind: "practice",
      difficulty: "beginner",
      summary: "Start a conveyor motor.",
      objectives: ["Latch a contactor"],
      minutes: 15,
      scene: "scene.motor-start-stop",
      platform: "generic.iec",
      requiredInterface: [
        { tag: "StartPB", dataType: "BOOL", direction: "input" },
      ],
      instructions: ["Pressing Start energizes MotorContactor."],
      hints: ["Use a seal-in."],
      knowledgeRefs: [],
      criteria: [
        { id: "starts", title: "Start energizes the contactor", points: 25 },
      ],
      troubleshooting: null,
    },
    frame: frame(),
    ...overrides,
  };
}

export const library: LibraryDto = {
  lessons: [
    {
      id: "lesson.01",
      number: 1,
      title: "What is a PLC?",
      summary: "The controller in the cabinet.",
      level: "beginner",
      minutes: 10,
      completed: true,
      checkpointCorrect: 2,
      checkpointTotal: 2,
    },
    {
      id: "lesson.02",
      number: 2,
      title: "Inputs and outputs",
      summary: "Signals in and out.",
      level: "beginner",
      minutes: 12,
      completed: false,
      checkpointCorrect: 0,
      checkpointTotal: 0,
    },
  ],
  exercises: [
    {
      id: "ex.motor-start-stop",
      title: "Motor start/stop with seal-in",
      kind: "practice",
      difficulty: "beginner",
      summary: "Start a motor.",
      minutes: 15,
      scene: "scene.motor-start-stop",
      platform: "generic.iec",
      attempts: 0,
      bestScore: 0,
      maxScore: 100,
      passed: false,
    },
    {
      id: "ts.motor-wont-start",
      title: "Conveyor will not start",
      kind: "troubleshooting",
      difficulty: "beginner",
      summary: "Find the fault.",
      minutes: 15,
      scene: "scene.motor-start-stop",
      platform: "generic.iec",
      attempts: 1,
      bestScore: 85,
      maxScore: 100,
      passed: true,
    },
  ],
  platforms: [],
  scenes: [
    {
      id: "scene.motor-start-stop",
      title: "Motor start/stop station",
      summary: "Motor.",
    },
  ],
};
