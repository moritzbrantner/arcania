// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  useBoardRendererDegradation,
  type BoardRendererDegradation,
} from "./boardRendererDegradation";
import type { BoardVisualMode } from "./types";

afterEach(() => cleanup());

describe("board renderer degradation", () => {
  it("starts in 2D fallback when WebGL is unavailable", () => {
    renderPolicy({ requestedMode: "3d", probeWebGL: () => false });

    expect(screen.getByTestId("renderer")).toHaveTextContent("2d");
    expect(screen.getByTestId("notice")).toHaveTextContent("3D board unavailable, using 2D.");
  });

  it("falls back after a fatal 3D render failure", () => {
    renderPolicy({ requestedMode: "3d", probeWebGL: () => true });
    expect(screen.getByTestId("renderer")).toHaveTextContent("3d");

    fireEvent.click(screen.getByRole("button", { name: "fatal" }));

    expect(screen.getByTestId("renderer")).toHaveTextContent("2d");
    expect(screen.getByTestId("notice")).toHaveTextContent("3D board failed to start, using 2D.");
  });

  it("counts asset failures and resets them for explicit 2D preference", async () => {
    const probeWebGL = vi.fn(() => true);
    const { rerender } = render(
      <PolicyProbe requestedMode="3d" readOnly={false} probeWebGL={probeWebGL} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "asset" }));
    fireEvent.click(screen.getByRole("button", { name: "asset" }));
    expect(screen.getByTestId("asset-failures")).toHaveTextContent("2");

    rerender(<PolicyProbe requestedMode="2d" readOnly={false} probeWebGL={probeWebGL} />);

    await waitFor(() => expect(screen.getByTestId("asset-failures")).toHaveTextContent("0"));
    expect(screen.getByTestId("renderer")).toHaveTextContent("2d");
    expect(screen.getByTestId("notice")).toBeEmptyDOMElement();
  });

  it("re-probes when switching from explicit 2D to requested 3D", async () => {
    const probeWebGL = vi.fn(() => true);
    const { rerender } = render(
      <PolicyProbe requestedMode="2d" readOnly={false} probeWebGL={probeWebGL} />,
    );

    expect(screen.getByTestId("renderer")).toHaveTextContent("2d");
    expect(probeWebGL).not.toHaveBeenCalled();

    rerender(<PolicyProbe requestedMode="3d" readOnly={true} probeWebGL={probeWebGL} />);

    await waitFor(() => expect(probeWebGL).toHaveBeenCalledOnce());
    expect(screen.getByTestId("renderer")).toHaveTextContent("3d");
  });
});

function renderPolicy({
  requestedMode,
  probeWebGL,
}: {
  requestedMode: BoardVisualMode;
  probeWebGL: () => boolean;
}) {
  return render(
    <PolicyProbe requestedMode={requestedMode} readOnly={false} probeWebGL={probeWebGL} />,
  );
}

function PolicyProbe({
  requestedMode,
  readOnly,
  probeWebGL,
}: {
  requestedMode: BoardVisualMode;
  readOnly: boolean;
  probeWebGL: () => boolean;
}) {
  const policy: BoardRendererDegradation = useBoardRendererDegradation({
    requestedMode,
    readOnly,
    probeWebGL,
  });

  return (
    <div>
      <span data-testid="renderer">{policy.renderer}</span>
      <span data-testid="notice">{policy.notice}</span>
      <span data-testid="asset-failures">{policy.assetFailureCount}</span>
      <button type="button" onClick={policy.reportFatalRenderFailure}>
        fatal
      </button>
      <button type="button" onClick={policy.reportAssetFailure}>
        asset
      </button>
    </div>
  );
}
