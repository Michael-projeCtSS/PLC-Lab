import { expect, it } from "vitest";
import type { DiagnosticEntryDto } from "@plc-lab/ipc";
import { translate } from ".";
import type { TranslationKey } from "./en";
import {
  backendText,
  diagnosticDetails,
  diagnosticSource,
  diagnosticText,
  diagnosticTitle,
  moduleStatus,
  moduleText,
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
    "Moduł skonfigurowany różni się od zamontowanego (skonfigurowany: brak, wykryty: Wejścia analogowe 4 x U/I).",
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

it("backend messages are worded by code, with terms, nested messages and lists in Polish", () => {
  const slot = {
    code: "hw.slot",
    text: "rack 0 slot 4",
    args: [
      { name: "rack", text: "0", term: null, message: null },
      { name: "slot", text: "4", term: null, message: null },
    ],
  };
  const wrongType = {
    code: "hw.wrongType",
    text: "ai-r0s4 at rack 0 slot 4: configured Analog input 4 x U/I, detected Digital input 8 x 24 V DC (sinking)",
    args: [
      { name: "id", text: "ai-r0s4", term: null, message: null },
      { name: "slot", text: slot.text, term: null, message: slot },
      {
        name: "name",
        text: "Analog input 4 x U/I",
        term: "module.ai4.ui",
        message: null,
      },
      {
        name: "detected",
        text: "Digital input 8 x 24 V DC (sinking)",
        term: "module.di8.24vdc.sink",
        message: null,
      },
    ],
  };
  expect(backendText("en", wrongType)).toBe(wrongType.text);
  expect(backendText("pl", wrongType)).toBe(
    "ai-r0s4 w pozycji kaseta 0, slot 4: skonfigurowano Wejścia analogowe 4 x U/I, wykryto Wejścia cyfrowe 8 x 24 V DC (sinking)",
  );
  const reasons = {
    code: "list",
    text: "no mains supply, faulted",
    args: [
      {
        name: "item",
        text: "no mains supply",
        term: "check.drive.noSupply",
        message: null,
      },
      {
        name: "item",
        text: "faulted",
        term: "check.drive.faulted",
        message: null,
      },
    ],
  };
  expect(
    backendText("pl", {
      code: "check.drive.notReady",
      text: "VFD: not ready (no mains supply, faulted)",
      args: [
        { name: "label", text: "VFD", term: null, message: null },
        { name: "reasons", text: reasons.text, term: null, message: reasons },
      ],
    }),
  ).toBe("VFD: niegotowy (brak zasilania sieciowego, usterka)");
  // A quality term is worded by the interface dictionary.
  expect(
    backendText("pl", {
      code: "check.in.quality",
      text: "%IX0.1 (StopPB): wire break",
      args: [
        { name: "channel", text: "%IX0.1 (StopPB)", term: null, message: null },
        {
          name: "quality",
          text: "wire break",
          term: "q.WIRE_BREAK",
          message: null,
        },
      ],
    }),
  ).toBe("%IX0.1 (StopPB): przerwanie przewodu");
  // Unknown codes keep the backend's English text; nothing is guessed.
  expect(
    backendText("pl", { code: "check.future", text: "Future text", args: [] }),
  ).toBe("Future text");
  expect(backendText("pl", null, "fallback")).toBe("fallback");
  expect(
    moduleText(
      (key, values) => translate("pl", key, values),
      "SM 1231 AI4 (Analog input 4 x U/I)",
    ),
  ).toBe("SM 1231 AI4 (Wejścia analogowe 4 x U/I)");
});
