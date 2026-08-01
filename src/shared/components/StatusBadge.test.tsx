import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusBadge } from "./StatusBadge";

describe("StatusBadge", () => {
  it("exposes the detected state as text rather than color alone", () => {
    render(<StatusBadge status="detected" />);
    expect(screen.getByText("Detected")).toBeVisible();
  });

  it("exposes unavailable state clearly", () => {
    render(<StatusBadge status="unavailable" />);
    expect(screen.getByText("Unavailable")).toBeVisible();
  });
});
