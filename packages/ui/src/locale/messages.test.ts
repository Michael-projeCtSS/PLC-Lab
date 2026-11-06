import { expect, it } from "vitest";
import type { DiagnosticEntryDto } from "@plc-lab/ipc";
import { translate } from ".";
import type { TranslationKey } from "./en";
import {
  diagnosticDetails,
  diagnosticSource,
  diagnosticText,
  diagnosticTitle,
  moduleStatus,
  qualityLabel,
} from "./messages";

const en = (key: TranslationKey, values?: Record<string, string | number>) =>
  translate("en", key, values);
const pl = (key: TranslationKey, values?: Record<string, string | number>) =>
  translate("pl", key, values);

function entry(
  kind: string,
  text: string,
  details: [string, string][] = [],
): DiagnosticEntryDto {
  return {
    timeUs: "0",
    source: "Rack 0 slot 2 channel 1 (%IW66)",
    kind,
    event: "INCOMING",
    text,
    slot: { rack: 0, slot: 2 },
    channel: { rack: 0, slot: 2, channel: 1 },
    moduleId: "ai",
    severity: "error",
    category: "HARDWARE",
    details: details.map(([name, value]) => ({ name, value })),
    active: true,
    clearedUs: null,
  };
}

// The backend's canonical English text (desktop-session convert::diagnostic_text) for
// the same kind and details: the English template must render it exactly.
const BACKEND: [DiagnosticEntryDto, string][] = [
  [
    entry("WireBreak", ""),
    "Wire break on analog input (loop current below the detection limit).",
  ],
  [
    entry("ModuleMismatch", "", [
      ["configured", "none"],
      ["installed", "Analog input 4 x U/I"],
    ]),
    "Configured module differs from installed module (configured none, found Analog input 4 x U/I).",
  ],
  [
    entry("ModbusException", "", [["exception", "0Bh"]]),
    "Modbus exception 0Bh: gateway target device failed to respond",
  ],
  [
    entry("ModeChange", "", [
      ["from", "RUN"],
      ["to", "STOP"],
    ]),
    "CPU operating state changed from RUN to STOP.",
  ],
  [
    entry("Startup", "", [
      ["start", "warm"],
      ["retained", "2"],
    ]),
    "Warm restart: non-retentive data initialized, 2 retentive variable(s) kept.",
  ],
  [
    entry("FirstScan", "", [["start", "cold"]]),
    "First program scan after the cold start executed.",
  ],
];

it("English diagnostic templates reproduce the backend's canonical text", () => {
  for (const [dto, text] of BACKEND) expect(diagnosticText(en, dto)).toBe(text);
});

it("Polish diagnostics are worded from the code and keep the technical values", () => {
  const [mismatch] = BACKEND[1]!;
  expect(diagnosticText(pl, mismatch)).toBe(
    "Moduł skonfigurowany różni się od zamontowanego (skonfigurowany: brak, wykryty: Analog input 4 x U/I).",
  );
  expect(diagnosticText(pl, BACKEND[2]![0])).toBe(
    "Wyjątek Modbus 0Bh: urządzenie docelowe za bramą nie odpowiedziało",
  );
  expect(
    diagnosticSource(pl, entry("WireBreak", "", [["address", "%IW66"]])),
  ).toBe("Kaseta 0, slot 2, kanał 1 (%IW66)");
  expect(diagnosticDetails(pl, entry("Startup", "", [["start", "warm"]]))).toBe(
    "start: ciepły",
  );
});

it("an unknown code falls back to the backend's English text, never to a guess", () => {
  expect(diagnosticText(pl, entry("FutureKind", "Original text"))).toBe(
    "Original text",
  );
  expect(diagnosticTitle(pl, "PLC-FUTURE-999")).toBe("");
  expect(diagnosticTitle(pl, "PLC-RUNTIME-003")).toBe(
    " · Dzielenie przez zero",
  );
  expect(moduleStatus(pl, "NEW_STATE")).toBe("NEW STATE");
  expect(qualityLabel(pl, "WIRE_BREAK")).toBe("przerwanie przewodu");
  expect(qualityLabel(en, "WIRE_BREAK")).toBe("wire break");
});
