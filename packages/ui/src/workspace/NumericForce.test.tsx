import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { NumericForce } from "./Dock";

it("forces INT values within the INT range and REAL values as typed", () => {
  const force = vi.fn();
  const { rerender } = render(<NumericForce valueType="INT" force={force} />);
  const input = screen.getByLabelText("Force value (INT)");
  const button = screen.getByRole("button", { name: "Force" });
  fireEvent.change(input, { target: { value: "40000" } });
  expect(button).toBeDisabled();
  fireEvent.change(input, { target: { value: "1.5" } });
  expect(button).toBeDisabled();
  fireEvent.change(input, { target: { value: "-1234" } });
  fireEvent.click(button);
  expect(force).toHaveBeenLastCalledWith({ type: "INT", value: -1234 });
  rerender(<NumericForce valueType="REAL" force={force} />);
  fireEvent.change(screen.getByLabelText("Force value (REAL)"), {
    target: { value: "12.75" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Force" }));
  expect(force).toHaveBeenLastCalledWith({ type: "REAL", value: 12.75 });
});
