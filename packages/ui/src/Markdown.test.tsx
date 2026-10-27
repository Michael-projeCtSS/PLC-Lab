import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { Markdown } from "./Markdown";

it("renders the supported subset as elements", () => {
  const { container } = render(
    <Markdown
      text={
        "## Title\n\nA **bold** and `code` text.\n\n- one\n- two\n\n1. first\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n```\nX := TRUE;\n```"
      }
    />,
  );
  expect(screen.getByRole("heading", { name: "Title" })).toBeInTheDocument();
  expect(container.querySelector("strong")?.textContent).toBe("bold");
  expect(container.querySelectorAll("li")).toHaveLength(3);
  expect(container.querySelector("td")?.textContent).toBe("1");
  expect(container.querySelector("pre")?.textContent).toBe("X := TRUE;");
});

it("never injects HTML or creates navigable links", () => {
  const { container } = render(
    <Markdown
      text={
        '<img src=x onerror="alert(1)"> [vendor](https://example.com/manual)'
      }
    />,
  );
  expect(container.querySelector("img")).toBeNull();
  expect(container.querySelector("a")).toBeNull();
  expect(container.textContent).toContain("<img");
  expect(container.textContent).toContain("https://example.com/manual");
});
