import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { StationDto } from "@plc-lab/ipc";
import { project, station } from "../../../../tests/frontend/fixtures";
import { NetworkConfig } from "./NetworkConfig";
import type { Lab } from "./useLab";

const networked: StationDto = {
  ...station,
  network: {
    interface: { ip: "192.168.0.10", mask: "255.255.255.0", gateway: null },
    modbusTcp: [],
  },
};
/** The same station after a saved hardware edit: the module moved to slot 5. */
const moved: StationDto = {
  ...networked,
  modules: networked.modules.map((m) => ({
    ...m,
    position: { rack: 0, slot: 5 },
  })),
  ioMap: networked.ioMap.map((m) => ({
    ...m,
    channel: { ...m.channel, slot: 5 },
  })),
};

function lab(s: StationDto, buildAndLoad: Lab["buildAndLoad"]): Lab {
  // The editor reads only these members of the lab.
  return {
    project: project({ station: s }),
    busy: null,
    buildAndLoad,
  } as unknown as Lab;
}

it("a network download keeps a hardware edit saved while the network draft was open", () => {
  const buildAndLoad = vi.fn().mockResolvedValue({ outcome: "LOADED" });
  const { rerender } = render(
    <NetworkConfig lab={lab(networked, buildAndLoad)} />,
  );
  fireEvent.change(screen.getByLabelText("PLC IP address"), {
    target: { value: "192.168.0.20" },
  });
  // A hardware edit is saved in the project while the network draft is open.
  rerender(<NetworkConfig lab={lab(moved, buildAndLoad)} />);
  fireEvent.click(
    screen.getByRole("button", { name: "Download configuration" }),
  );
  const sent = buildAndLoad.mock.calls[0]![1] as StationDto;
  expect(sent.modules[0]!.position.slot).toBe(5);
  expect(sent.ioMap[0]!.channel.slot).toBe(5);
  expect(sent.network.interface?.ip).toBe("192.168.0.20");
});

it("a network download after a saved hardware edit sends both", () => {
  const buildAndLoad = vi.fn().mockResolvedValue({ outcome: "LOADED" });
  render(<NetworkConfig lab={lab(moved, buildAndLoad)} />);
  fireEvent.change(screen.getByLabelText("PLC IP address"), {
    target: { value: "192.168.0.30" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Download configuration" }),
  );
  const sent = buildAndLoad.mock.calls[0]![1] as StationDto;
  expect(sent.modules).toEqual(moved.modules);
  expect(sent.network.interface?.ip).toBe("192.168.0.30");
});
