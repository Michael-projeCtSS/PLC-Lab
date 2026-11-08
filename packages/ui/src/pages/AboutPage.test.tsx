import { render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import type { CoreInfo } from "@plc-lab/ipc";
import { Context } from "../locale/context";
import { defaults } from "../locale/preferences";

const fake = vi.hoisted(() => ({
  coreInfo: vi.fn(),
  knowledgeStatus: vi.fn(),
}));
vi.mock("@plc-lab/ipc", async (original) => ({
  ...(await original<typeof import("@plc-lab/ipc")>()),
  api: fake,
}));

import { AboutPage } from "./AboutPage";

const info = (overrides: Partial<CoreInfo>): CoreInfo => ({
  applicationName: "PLC Lab",
  appVersion: "0.2.0-dev",
  coreVersion: "0.2.0-dev",
  contractVersion: 8,
  execution: "manual-and-continuous",
  buildContext: "release",
  sourceCommit: "d950c2f584491f803cb1e45e954576acf897654f",
  sourceDirty: false,
  builtAt: "2026-11-08T20:38:54Z",
  distribution: "UNSIGNED_DEVELOPMENT_BUILD",
  signed: false,
  ...overrides,
});

function show(language: "en" | "pl") {
  return render(
    <Context.Provider
      value={{
        preferences: defaults(language),
        update: vi.fn(),
        problem: null,
        theme: "dark",
      }}
    >
      <AboutPage />
    </Context.Provider>,
  );
}

beforeEach(() => {
  fake.knowledgeStatus.mockResolvedValue({
    packs: [],
    searchPaths: [],
    indexedItems: 0,
  });
});

it("names the exact source commit, the distribution and the signing state", async () => {
  fake.coreInfo.mockResolvedValue(info({}));
  show("en");
  const commit = await screen.findByTestId("about-commit");
  expect(commit).toHaveTextContent("d950c2f58449");
  expect(commit).toHaveAttribute(
    "title",
    "d950c2f584491f803cb1e45e954576acf897654f",
  );
  expect(screen.getByTestId("about-distribution")).toHaveTextContent(
    "Unsigned development build (UPDATE_PLC_LAB.bat)",
  );
  expect(screen.getByText("Not code-signed")).toBeInTheDocument();
  expect(screen.queryByRole("note")).toBeNull();
});

it("warns about uncommitted changes and says what is unknown, in Polish too", async () => {
  fake.coreInfo.mockResolvedValue(
    info({
      sourceCommit: null,
      sourceDirty: true,
      builtAt: null,
      distribution: "DEVELOPMENT_RUN",
    }),
  );
  show("pl");
  expect(await screen.findByTestId("about-commit")).toHaveTextContent(
    "nieznany",
  );
  expect(screen.getByTestId("about-distribution")).toHaveTextContent(
    "Uruchomienie deweloperskie",
  );
  expect(screen.getByRole("note")).toHaveTextContent(
    "niezatwierdzonymi zmianami",
  );
});

it("a signed release and a foreign manifest are labelled as such", async () => {
  fake.coreInfo.mockResolvedValue(
    info({ distribution: "SIGNED_PRODUCTION_RELEASE", signed: true }),
  );
  const { unmount } = show("en");
  expect(
    await screen.findByText("Signed (verified when the release was staged)"),
  ).toBeInTheDocument();
  unmount();
  fake.coreInfo.mockResolvedValue(info({ distribution: "MANIFEST_MISMATCH" }));
  show("en");
  expect(await screen.findByTestId("about-distribution")).toHaveTextContent(
    "describes another build",
  );
});
