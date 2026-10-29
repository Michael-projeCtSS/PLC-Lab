/** Validators for every native response DTO of contract v8. */
import type * as G from "./generated";
import {
  arr,
  bool,
  i16,
  int,
  literal,
  nullable,
  num,
  obj,
  safeInt,
  str,
  tagged,
  u16,
  u32,
  u64,
  u8,
  type Validator,
} from "./validate";

const u16n = u16;

export const signalValue: Validator<G.SignalValueDto> =
  tagged<G.SignalValueDto>({
    BOOL: obj({ type: literal("BOOL"), value: bool }),
    INT: obj({ type: literal("INT"), value: i16 }),
    REAL: obj({ type: literal("REAL"), value: num }),
    TIME: obj({ type: literal("TIME"), value: u64 }),
    COUNT: obj({ type: literal("COUNT"), value: safeInt }),
  });

const position = obj<G.PositionDto>({ line: u32, column: u32 });
const span = obj<G.SpanDto>({ start: position, end: position });
export const diagnostic = obj<G.DiagnosticDto>({
  code: str,
  severity: literal("error", "warning", "info"),
  message: str,
  span: nullable(span),
  category: literal("compiler", "runtime", "internal"),
});

export const coreInfo = obj<G.CoreInfo>({
  applicationName: str,
  appVersion: str,
  coreVersion: str,
  contractVersion: int(8, 8),
  execution: str,
  buildContext: str,
});

// ---- knowledge -------------------------------------------------------------------

const pack = obj<G.PackDto>({
  id: nullable(str),
  title: nullable(str),
  version: nullable(str),
  path: str,
  status: literal("LOADED", "PARTIAL", "INVALID"),
  errors: arr(str),
  warnings: arr(str),
  articles: u32,
  lessons: u32,
  exercises: u32,
  scenes: u32,
  sources: u32,
  profiles: u32,
});
export const knowledgeStatus = obj<G.KnowledgeStatusDto>({
  packs: arr(pack),
  indexedItems: u32,
  searchPaths: arr(str),
});
export const searchHits = arr(
  obj<G.SearchHitDto>({
    id: str,
    docType: str,
    category: str,
    title: str,
    summary: str,
    vendor: nullable(str),
    snippet: str,
  }),
  100,
);
const related = obj<G.RelatedDto>({ id: str, title: str });
const fidelity = obj<G.FidelityDto>({
  level: str,
  modeled: arr(str),
  approximated: arr(str),
  omitted: arr(str),
});
const parameter = obj<G.ParameterDto>({
  name: str,
  dataType: str,
  description: str,
});
export const article = obj<G.ArticleDto>({
  id: str,
  kind: str,
  category: str,
  title: str,
  summary: str,
  vendor: nullable(str),
  family: nullable(str),
  protocol: nullable(str),
  tier: nullable(str),
  tags: arr(str),
  sections: arr(obj<G.SectionDto>({ heading: str, body: str })),
  facts: arr(obj<G.FactDto>({ label: str, value: str, sources: arr(str) })),
  fidelity: nullable(fidelity),
  instruction: nullable(
    obj<G.InstructionDto>({
      category: str,
      inputs: arr(parameter),
      outputs: arr(parameter),
      state: str,
      retentivity: str,
      timing: str,
      simulationSupport: str,
      equivalents: arr(
        obj<G.EquivalentDto>({ platform: str, name: str, notes: str }),
      ),
    }),
  ),
  troubleshooting: nullable(
    obj<G.TroubleshootingDto>({
      faultId: str,
      area: str,
      target: str,
      effect: str,
      symptoms: arr(str),
      evidence: arr(str),
      diagnosticMethod: arr(str),
      repair: arr(str),
      explanation: str,
      simulatedIn: arr(related),
    }),
  ),
  simulationProfile: nullable(str),
  sources: arr(
    obj<G.SourceDto>({
      id: str,
      vendor: nullable(str),
      title: str,
      documentType: str,
      documentNumber: nullable(str),
      url: str,
      retrieved: str,
      appliesTo: arr(str),
      notes: str,
      confidence: str,
      verification: str,
    }),
  ),
  related: arr(related),
});
const catalogEntry: Validator<G.CatalogEntryDto> = (v, p) =>
  obj<G.CatalogEntryDto>({
    id: str,
    title: str,
    summary: str,
    tier: nullable(str),
    profile: nullable(str),
    area: nullable(str),
    children: arr(catalogEntry, 500),
  })(v, p);
export const catalog = obj<G.CatalogDto>({
  vendors: arr(catalogEntry),
  protocols: arr(catalogEntry),
  fundamentals: arr(catalogEntry),
  instructions: arr(catalogEntry),
  troubleshooting: arr(catalogEntry),
});
const question = obj<G.QuestionDto>({
  question: str,
  options: arr(str, 12),
  answer: u8,
  explanation: str,
});
export const lesson = obj<G.LessonDto>({
  id: str,
  number: u16n,
  title: str,
  summary: str,
  level: str,
  minutes: u16n,
  objectives: arr(str),
  sections: arr(
    obj<G.LessonSectionDto>({
      sectionType: str,
      heading: str,
      body: str,
      code: nullable(str),
      scene: nullable(str),
      exercise: nullable(str),
      question: nullable(question),
      diagram: nullable(str),
    }),
  ),
  checkpoint: arr(question),
  knowledgeRefs: arr(related),
});
const difficulty = literal("beginner", "intermediate", "advanced");
const exerciseKind = literal("practice", "troubleshooting", "commissioning");
const option = obj<G.OptionDto>({ id: str, label: str });
export const exercise = obj<G.ExerciseDto>({
  id: str,
  revision: u32,
  title: str,
  kind: exerciseKind,
  difficulty,
  summary: str,
  objectives: arr(str),
  minutes: u16n,
  scene: str,
  platform: str,
  requiredInterface: arr(
    obj<G.RequiredTagDto>({ tag: str, dataType: str, direction: str }),
  ),
  instructions: arr(str),
  hints: arr(str),
  knowledgeRefs: arr(related),
  criteria: arr(obj<G.CriterionDto>({ id: str, title: str, points: u32 })),
  troubleshooting: nullable(
    obj<G.TroubleshootingBriefDto>({
      symptom: str,
      reportedBy: str,
      diagnosisOptions: arr(option),
      diagnosed: bool,
      repairs: arr(option),
    }),
  ),
});
export const platform = obj<G.PlatformDto>({
  id: str,
  displayName: str,
  vendor: str,
  family: str,
  tier: str,
  fidelity,
  addressing: str,
  analog: str,
});
export const library = obj<G.LibraryDto>({
  lessons: arr(
    obj<G.LessonSummaryDto>({
      id: str,
      number: u16n,
      title: str,
      summary: str,
      level: str,
      minutes: u16n,
      completed: bool,
      checkpointCorrect: u16n,
      checkpointTotal: u16n,
    }),
  ),
  exercises: arr(
    obj<G.ExerciseSummaryDto>({
      id: str,
      title: str,
      kind: exerciseKind,
      difficulty,
      summary: str,
      minutes: u16n,
      scene: str,
      platform: str,
      attempts: u32,
      bestScore: u32,
      maxScore: u32,
      passed: bool,
    }),
  ),
  platforms: arr(platform),
  scenes: arr(obj<G.SceneSummaryDto>({ id: str, title: str, summary: str })),
  notices: arr(str),
});

// ---- projects ----------------------------------------------------------------------

const slotRef = obj<G.SlotRefDto>({ rack: u8, slot: u8 });
const channelRef = obj<G.ChannelRefDto>({ rack: u8, slot: u8, channel: u8 });
const interfaceDto = obj<G.InterfaceDto>({
  ip: str,
  mask: str,
  gateway: nullable(str),
});
export const station = obj<G.StationDto>({
  name: str,
  cpu: str,
  modules: arr(
    obj<G.ConfiguredModuleDto>({
      id: str,
      position: slotRef,
      module: str,
      startAddress: u16,
      channels: arr(
        obj<G.ChannelConfigDto>({
          channel: u8,
          mode: literal(
            "disabled",
            "digital",
            "current4To20mA",
            "current0To20mA",
            "voltage0To10V",
          ),
          diagnostics: bool,
          stopBehavior: literal("off", "holdLast"),
          loopSupply: bool,
        }),
      ),
    }),
  ),
  ioMap: arr(obj<G.IoMappingDto>({ channel: channelRef, tag: str })),
  network: obj<G.StationNetworkDto>({
    interface: nullable(interfaceDto),
    modbusTcp: arr(
      obj<G.ModbusScanDto>({
        id: str,
        server: str,
        port: u16,
        unitId: u8,
        function: literal("readHoldingRegisters", "readInputRegisters"),
        startAddress: u16,
        quantity: u16,
        periodMs: u32,
        timeoutMs: u32,
        values: arr(
          obj<G.ModbusValueDto>({
            offset: u16,
            dataType: tagged<G.ModbusDataTypeDto>({
              int16: obj({ type: literal("int16") }),
              float32: obj({ type: literal("float32"), wordOrder: str }),
            }),
            tag: str,
          }),
        ),
        statusTag: nullable(str),
      }),
    ),
  }),
});
const projectKind = literal("freeLab", "exercise");
const watchTable = obj<G.WatchTableDto>({
  id: str,
  name: str,
  rows: arr(
    obj<G.WatchRowDto>({
      signal: str,
      format: literal("auto", "decimal", "hex", "binary"),
    }),
    64,
  ),
});
export const project = obj<G.ProjectDto>({
  id: str,
  name: str,
  kind: projectKind,
  exercise: nullable(str),
  platform: str,
  scene: str,
  station,
  programs: arr(
    obj<G.ProgramDocumentDto>({
      id: str,
      name: str,
      language: str,
      source: str,
    }),
  ),
  watchTables: arr(watchTable, 16),
  simulation: obj<G.SimulationSettingsDto>({ cycleMs: u32, speedPercent: u32 }),
  createdAt: str,
  updatedAt: str,
});
export const projectSummary = obj<G.ProjectSummaryDto>({
  id: str,
  name: str,
  kind: projectKind,
  exercise: nullable(str),
  platform: str,
  scene: str,
  updatedAt: str,
});
export const projectList = obj<G.ProjectListDto>({
  projects: arr(projectSummary),
  problems: arr(str),
  directory: str,
});
export const projectExport = obj<G.ProjectExportDto>({
  fileName: str,
  text: str,
});

// ---- workspace -----------------------------------------------------------------------

export const signal = obj<G.SignalDto>({
  id: u32,
  path: str,
  label: str,
  kind: literal(
    "PLC_INPUT",
    "PLC_OUTPUT",
    "PLC_LOCAL",
    "BLOCK_OUTPUT",
    "PHYSICAL",
    "CHANNEL",
  ),
  valueType: literal("BOOL", "INT", "REAL", "TIME", "COUNT"),
  unit: nullable(str),
  active: bool,
});
const led = obj<G.LedDto>({
  name: str,
  state: literal(
    "OFF",
    "GREEN",
    "GREEN_FLASHING",
    "RED",
    "RED_FLASHING",
    "AMBER",
  ),
});
const effect = obj<G.EffectDto>({
  signal: u32,
  value: signalValue,
  changed: bool,
});
export const causeEffect = obj<G.CauseEffectDto>({
  cycle: u64,
  startUs: u64,
  endUs: u64,
  segment: u32,
  outcome: literal(
    "SCANNED",
    "FAULTED",
    "NOT_SCANNED_STOP",
    "NOT_SCANNED_FAULT",
  ),
  saw: arr(effect),
  decided: arr(effect),
  applied: arr(effect),
  process: arr(
    obj<G.ChangeDto>({
      signal: u32,
      before: nullable(signalValue),
      after: signalValue,
    }),
  ),
  next: arr(effect),
  fault: nullable(diagnostic),
});
export const frame = obj<G.LiveFrameDto>({
  workspaceId: u64,
  worldTimeUs: u64,
  cycle: u64,
  simulation: obj<G.SimulationStatusDto>({
    state: literal("RUNNING", "PAUSED", "STOPPED"),
    speedPercent: u32,
    runId: nullable(u64),
    cycleUs: u64,
    failure: nullable(str),
  }),
  plc: obj<G.PlcStatusDto>({
    mode: literal("RUN", "STOP", "FAULT"),
    switch: literal("RUN", "STOP"),
    programName: nullable(str),
    programRevision: nullable(u64),
    segment: u32,
    scans: u64,
    taskTimeUs: u64,
    fault: nullable(diagnostic),
    forces: u32,
    leds: arr(led),
  }),
  components: arr(
    obj<G.ComponentStateDto>({
      id: str,
      quantities: arr(obj<G.QuantityDto>({ name: str, value: signalValue })),
    }),
  ),
  watch: arr(
    obj<G.WatchValueDto>({ signal: u32, value: nullable(signalValue) }),
    64,
  ),
  causeEffect: nullable(causeEffect),
  alarms: arr(obj<G.AlarmDto>({ component: str, kind: str, text: str })),
  activeDiagnostics: u32,
  latestEvent: nullable(u64),
  trace: obj<G.TraceStatsDto>({
    retainedCycles: u64,
    droppedCycles: u64,
    usedBytes: u64,
    budgetBytes: u64,
  }),
  signalsRevision: u32,
});
export const workspace = obj<G.WorkspaceDto>({
  workspaceId: u64,
  projectId: str,
  scene: obj<G.SceneDto>({
    id: str,
    title: str,
    summary: str,
    components: arr(
      obj<G.ComponentDto>({
        id: str,
        label: str,
        kind: str,
        color: nullable(str),
        positions: arr(str),
        operable: bool,
        config: arr(obj<G.ConfigItemDto>({ key: str, value: str })),
      }),
    ),
    layout: arr(
      obj<G.LayoutDto>({
        component: str,
        x: u16,
        y: u16,
        width: nullable(u16),
        height: nullable(u16),
      }),
    ),
    couplings: arr(
      obj<G.CouplingDto>({
        kind: literal("valveFillsTank", "valveDrainsTank", "pumpDrainsTank"),
        from: str,
        to: str,
      }),
    ),
    interface: arr(
      obj<G.InterfaceSignalDto>({
        tag: str,
        dataType: str,
        direction: str,
        description: str,
      }),
    ),
    hasNetwork: bool,
  }),
  platform,
  signals: arr(signal),
  exercise: nullable(exercise),
  frame,
});
export const compileResult = obj<G.CompileResultDto>({
  success: bool,
  programName: nullable(str),
  diagnostics: arr(diagnostic),
  declarations: arr(
    obj<G.DeclarationDto>({ name: str, dataType: str, role: str }),
  ),
});
export const buildLoadResult = obj<G.BuildLoadResultDto>({
  outcome: literal(
    "LOADED",
    "COMPILE_FAILED",
    "BINDING_FAILED",
    "HARDWARE_REJECTED",
    "SUPERSEDED",
  ),
  diagnostics: arr(diagnostic),
  bindingIssues: arr(
    obj<G.BindingIssueDto>({
      tag: str,
      location: str,
      message: str,
      blocking: bool,
    }),
  ),
  message: nullable(str),
  signals: arr(signal),
  frame,
});
export const variables = obj<G.VariablesDto>({
  total: u32,
  rows: arr(
    obj<G.VariableRowDto>({
      signal: u32,
      name: str,
      kind: str,
      scanValue: nullable(signalValue),
      committed: nullable(signalValue),
      staged: nullable(signalValue),
      source: nullable(str),
      quality: nullable(str),
      forced: nullable(signalValue),
    }),
    500,
  ),
});
export const watch = obj<G.WatchDto>({
  worldTimeUs: u64,
  cycle: u64,
  rows: arr(
    obj<G.WatchEntryDto>({
      signal: u32,
      path: str,
      label: str,
      kind: str,
      valueType: str,
      unit: nullable(str),
      active: bool,
      value: nullable(signalValue),
      scanValue: nullable(signalValue),
      committed: nullable(signalValue),
      staged: nullable(signalValue),
      source: nullable(str),
      quality: nullable(str),
      forced: nullable(signalValue),
    }),
    64,
  ),
});
export const trend = obj<G.TrendDto>({
  series: arr(
    obj<G.TrendSeriesDto>({
      signal: u32,
      layer: literal("INPUTS", "COMMITTED", "PROCESS"),
      points: arr(obj<G.TrendPointDto>({ t: u64, value: signalValue }), 5_000),
      decimated: bool,
    }),
    16,
  ),
  oldestUs: nullable(u64),
  latestUs: nullable(u64),
});
const event = obj<G.EventDto>({
  seq: u64,
  timeUs: u64,
  cycle: u64,
  kind: str,
  severity: literal("info", "warning", "error"),
  text: str,
  source: nullable(str),
});
export const eventsPage = obj<G.EventsPageDto>({
  events: arr(event, 500),
  gap: bool,
  dropped: u64,
  latest: nullable(u64),
});
export const hardware = obj<G.HardwareDto>({
  station,
  cpu: str,
  cpuName: str,
  cpuProduct: nullable(str),
  cpuLeds: arr(led),
  modules: arr(
    obj<G.ModuleViewDto>({
      moduleId: nullable(str),
      position: slotRef,
      configured: nullable(str),
      installed: nullable(str),
      configuredName: nullable(str),
      installedName: nullable(str),
      product: nullable(str),
      status: literal(
        "OK",
        "MISSING",
        "MISMATCH",
        "NO_FIELD_POWER",
        "NO_MODULE_POWER",
        "UNCONFIGURED",
      ),
      statusLed: str,
      channels: arr(
        obj<G.ChannelViewDto>({
          channelId: nullable(str),
          channel: u8,
          mode: str,
          tag: nullable(str),
          address: str,
          value: nullable(signalValue),
          quality: nullable(str),
          output: nullable(bool),
          led: bool,
        }),
      ),
    }),
  ),
  diagnostics: arr(
    obj<G.DiagnosticEntryDto>({
      timeUs: u64,
      source: str,
      kind: str,
      event: literal("INCOMING", "OUTGOING"),
      text: str,
      slot: nullable(slotRef),
      channel: nullable(channelRef),
      moduleId: nullable(str),
    }),
  ),
  catalog: arr(
    obj<G.ModuleTypeDto>({
      id: str,
      name: str,
      product: nullable(str),
      channels: u8,
      class: str,
    }),
  ),
  findings: arr(
    obj<G.HardwareFindingDto>({
      category: literal(
        "MODULE_MISSING",
        "UNEXPECTED_MODULE",
        "WRONG_MODULE_TYPE",
        "WRONG_SLOT",
        "MODULE_POWER_MISSING",
        "FIELD_POWER_MISSING",
        "CHANNEL_MODE_MISMATCH",
        "LOOP_SUPPLY_MISMATCH",
        "ADDRESS_OVERLAP",
        "SIGNAL_BINDING_MISMATCH",
      ),
      severity: literal("warning", "error"),
      slot: nullable(slotRef),
      channel: nullable(channelRef),
      moduleId: nullable(str),
      expected: nullable(str),
      detected: nullable(str),
      message: str,
    }),
    2_000,
  ),
  signals: arr(
    obj<G.SignalMapRowDto>({
      tag: str,
      channel: channelRef,
      channelId: nullable(str),
      address: str,
      value: nullable(signalValue),
      quality: nullable(str),
      output: nullable(bool),
      drawnDevice: nullable(str),
      drawnPort: nullable(str),
      deviceState: nullable(str),
    }),
    4_096,
  ),
});
const checkState = literal(
  "OK",
  "ATTENTION",
  "FAULT",
  "UNTESTED",
  "NOT_APPLICABLE",
);
const checkItem = obj<G.CheckItemDto>({
  id: str,
  title: str,
  state: checkState,
  findings: arr(str),
  how: str,
});
const checkCategory = obj<G.CheckCategoryDto>({
  id: literal(
    "power",
    "hardware",
    "network",
    "inputs",
    "outputs",
    "drives",
    "process",
  ),
  title: str,
  state: checkState,
  items: arr(checkItem),
});
export const commissioning = obj<G.CommissioningDto>({
  categories: arr(checkCategory),
  feedbackWaitS: u32,
});

export const network = obj<G.NetworkDto>({
  plcInterface: nullable(interfaceDto),
  plcLinkUp: bool,
  scans: arr(
    obj<G.ScanViewDto>({
      id: str,
      server: str,
      unitId: u8,
      status: str,
      connected: bool,
      requests: u64,
      responses: u64,
      timeouts: u64,
      exceptions: u64,
      lastRoundTripUs: nullable(u64),
      registers: arr(u16, 125),
      request: obj<G.ModbusRequestDto>({
        function: literal("readHoldingRegisters", "readInputRegisters"),
        functionCode: u8,
        startAddress: u16,
        quantity: u16,
        port: u16,
        periodMs: u32,
        timeoutMs: u32,
      }),
      values: arr(
        obj<G.ScanValueDto>({
          tag: str,
          offset: u16,
          dataType: literal("int16", "float32"),
          wordOrder: nullable(literal("highWordFirst", "lowWordFirst")),
          value: nullable(str),
        }),
      ),
      lastRequestUs: nullable(u64),
      lastResponseUs: nullable(u64),
    }),
  ),
  devices: arr(
    obj<G.DeviceViewDto>({
      id: str,
      label: str,
      ip: str,
      mask: str,
      linkUp: bool,
      online: bool,
      modbusServer: bool,
      requestsServed: u64,
      lastActivityUs: nullable(u64),
      sameSubnet: nullable(bool),
    }),
  ),
  traffic: arr(
    obj<G.TrafficDto>({
      seq: u64,
      timeUs: u64,
      connection: str,
      transaction: u16,
      peer: str,
      kind: literal(
        "request",
        "response",
        "exception",
        "timeout",
        "noRoute",
        "arpUnresolved",
        "refused",
        "addressConflict",
      ),
      unitId: nullable(u8),
      functionCode: nullable(u8),
      address: nullable(u16),
      quantity: nullable(u16),
      roundTripUs: nullable(u64),
      exceptionCode: nullable(u8),
    }),
    60,
  ),
});
export const measurement = obj<G.MeasurementDto>({
  kind: literal("VOLTAGE", "CURRENT", "NO_MODULE"),
  value: safeInt,
  text: str,
});
export const ping = obj<G.PingDto>({
  result: str,
  roundTripUs: nullable(u64),
  text: str,
});
export const diagnosis = obj<G.DiagnosisResultDto>({
  correct: bool,
  message: str,
  attempts: u32,
  repairs: arr(option),
});
export const repair = obj<G.RepairResultDto>({ message: str, frame });
export const gradeReport = obj<G.GradeReportDto>({
  outcome: literal(
    "GRADED",
    "COMPILE_FAILED",
    "INTERFACE_MISMATCH",
    "BINDING_FAILED",
  ),
  score: u32,
  maxScore: u32,
  passed: bool,
  criteria: arr(
    obj<G.CriterionResultDto>({
      id: str,
      title: str,
      points: u32,
      earned: u32,
      passed: bool,
      feedback: str,
      detail: str,
    }),
  ),
  diagnostics: arr(diagnostic),
  problems: arr(str),
  debrief: nullable(
    obj<G.DebriefDto>({
      rootCause: str,
      evidence: arr(str),
      repair: str,
      reasoning: str,
      references: arr(related),
    }),
  ),
});
export const openRequest = nullable(
  obj<G.OpenRequestDto>({
    projectId: nullable(str),
    name: nullable(str),
    problem: nullable(str),
  }),
);
export const openRequestEvent = obj<G.OpenRequestEventDto>({
  contractVersion: int(8, 8),
});
export const executionWake = obj<G.ExecutionWakeDto>({
  contractVersion: int(8, 8),
  workspaceId: u64,
  runId: u64,
  cycle: u64,
});
