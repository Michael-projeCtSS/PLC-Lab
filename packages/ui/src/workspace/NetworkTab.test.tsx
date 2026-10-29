import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { NetworkDto, ScanViewDto } from "@plc-lab/ipc";
import { frame, workspace } from "../../../../tests/frontend/fixtures";
import type { Lab } from "./useLab";

const fake = vi.hoisted(() => ({
  network: vi.fn(),
  noteEvidence: vi.fn(),
  ping: vi.fn(),
}));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { NetworkTab } from "./NetworkTab";

function scan(overrides: Partial<ScanViewDto>): ScanViewDto {
  return {
    id: "li402",
    server: "192.168.10.30",
    unitId: 5,
    status: "OK",
    connected: true,
    requests: "12",
    responses: "12",
    timeouts: "0",
    exceptions: "0",
    lastRoundTripUs: "40800",
    registers: [16800, 0],
    request: {
      function: "readHoldingRegisters",
      functionCode: 3,
      startAddress: 100,
      quantity: 2,
      port: 502,
      periodMs: 200,
      timeoutMs: 300,
    },
    values: [
      {
        tag: "LevelLitres",
        offset: 0,
        dataType: "float32",
        wordOrder: "highWordFirst",
        value: "20",
      },
    ],
    lastRequestUs: "2400000",
    lastResponseUs: "2440800",
    ...overrides,
  };
}

const network: NetworkDto = {
  plcInterface: { ip: "192.168.10.10", mask: "255.255.255.0", gateway: null },
  plcLinkUp: true,
  scans: [
    scan({
      id: "lt401",
      server: "192.168.10.22",
      unitId: 1,
      status: "TIMEOUT",
    }),
    scan({}),
  ],
  devices: [
    {
      id: "gw-402",
      label: "Gateway GW-402",
      ip: "192.168.10.30",
      mask: "255.255.255.0",
      linkUp: true,
      online: true,
      modbusServer: true,
      requestsServed: "12",
      lastActivityUs: "2440000",
      sameSubnet: true,
    },
  ],
  traffic: [
    {
      seq: "1",
      timeUs: "2400000",
      connection: "li402",
      transaction: 7,
      peer: "192.168.10.30",
      kind: "request",
      unitId: 5,
      functionCode: 3,
      address: 100,
      quantity: 2,
      roundTripUs: null,
      exceptionCode: null,
    },
    {
      seq: "2",
      timeUs: "2401000",
      connection: "li402",
      transaction: 7,
      peer: "192.168.10.30",
      kind: "exception",
      unitId: null,
      functionCode: null,
      address: null,
      quantity: null,
      roundTripUs: null,
      exceptionCode: 11,
    },
  ],
};

function lab(): Lab {
  // The view reads only these members of the lab.
  return {
    ws: { workspaceId: "1" },
    frame: frame(),
    workspace: workspace(),
    log: vi.fn(),
  } as unknown as Lab;
}

it("inspects a connection and shows the traffic the backend reported", async () => {
  fake.network.mockResolvedValue(network);
  fake.noteEvidence.mockResolvedValue(null);
  render(<NetworkTab lab={lab()} advanced={false} />);
  const nodes = await screen.findByRole("cell", { name: "Gateway GW-402" });
  expect(within(nodes.closest("tr")!).getByText("yes")).toBeInTheDocument();
  // The first connection is inspected; selecting the gateway shows its request.
  expect(
    screen.getByRole("region", { name: "Connection lt401" }),
  ).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "li402" }));
  const inspector = screen.getByRole("region", { name: "Connection li402" });
  expect(
    within(inspector).getByText("100 … 101 (2 registers)"),
  ).toBeInTheDocument();
  expect(within(inspector).getByText("40101 … 40102")).toBeInTheDocument();
  expect(within(inspector).getByText("41A0h")).toBeInTheDocument();
  expect(
    within(inspector).getByText("REAL (32 bit, high word first)"),
  ).toBeInTheDocument();
  const traffic = screen.getByRole("table", { name: "Modbus traffic" });
  expect(
    within(traffic).getByText(
      "FC03 read 2 register(s) from address 100, unit 5, to 192.168.10.30",
    ),
  ).toBeInTheDocument();
  expect(
    within(traffic).getByText(
      "exception 0Bh: gateway target device failed to respond from 192.168.10.30",
    ),
  ).toBeInTheDocument();
});
