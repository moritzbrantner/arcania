// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { BoxGeometry, Mesh, MeshBasicMaterial, Object3D } from "three";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import * as browserRenderer from "./boardRenderer";
import { HeroPreview3D } from "./HeroPreview3D";
import { BOARD_VISUAL_MODE_STORAGE_KEY } from "./boardVisualMode";

const renderer = vi.hoisted(() => ({ fail: false, mountScene: false }));
vi.mock("@react-three/fiber", () => ({
  Canvas: ({ children }: { children: ReactNode }) => {
    if (renderer.fail) throw new Error("Renderer startup failed");
    return renderer.mountScene
      ? <div aria-label="Hero renderer">{children}</div>
      : <canvas aria-label="Hero renderer" />;
  },
}));

const assetLoad = vi.hoisted(() => vi.fn<(
  path: string,
  onLoad: (value: { scene: Object3D }) => void,
  onProgress: unknown,
  onError: (error: Error) => void,
) => void>());
vi.mock("three/examples/jsm/loaders/GLTFLoader.js", () => ({
  GLTFLoader: class { load = assetLoad; },
}));

beforeEach(() => {
  localStorage.setItem(BOARD_VISUAL_MODE_STORAGE_KEY, "3d");
  renderer.fail = false;
  renderer.mountScene = false;
  assetLoad.mockReset();
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  vi.restoreAllMocks();
});

it("keeps a Hero portrait available when WebGL cannot start", () => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
  render(<HeroPreview3D heroType="runekeeper" label="Runekeeper" />);
  expect(screen.getByRole("img", { name: "Runekeeper portrait" })).toHaveAttribute(
    "src", "/hero-art/runekeeper.svg",
  );
  expect(screen.queryByLabelText("Hero renderer")).not.toBeInTheDocument();
});

it("keeps a Hero portrait available after renderer startup throws", () => {
  vi.spyOn(browserRenderer, "canCreateWebGLContext").mockReturnValue(true);
  const errors = vi.spyOn(console, "error").mockImplementation(() => undefined);
  renderer.fail = true;
  expect(() => render(<HeroPreview3D heroType="pyromancer" label="Pyromancer" />)).not.toThrow();
  expect(screen.getByRole("img", { name: "Pyromancer portrait" })).toBeInTheDocument();
  expect(errors).toHaveBeenCalled();
});

it("uses the existing 2D preference without starting a renderer", () => {
  localStorage.setItem(BOARD_VISUAL_MODE_STORAGE_KEY, "2d");
  const probe = vi.spyOn(browserRenderer, "canCreateWebGLContext");
  render(<HeroPreview3D heroType="warden" label="Warden" />);
  expect(screen.getByRole("img", { name: "Warden portrait" })).toBeInTheDocument();
  expect(probe).not.toHaveBeenCalled();
});

it("replaces the procedural miniature with a cloned configured model", async () => {
  vi.spyOn(browserRenderer, "canCreateWebGLContext").mockReturnValue(true);
  renderer.mountScene = true;
  const scene = new Object3D();
  scene.add(new Mesh(new BoxGeometry(1, 2, 1), new MeshBasicMaterial()));
  assetLoad.mockImplementationOnce((_path, onLoad) => onLoad({ scene }));
  const { container } = render(<HeroPreview3D heroType="runekeeper" label="Runekeeper" />);
  await waitFor(() => expect(container.querySelector("primitive")).toBeInTheDocument());
  expect(container.querySelector("mesh")).not.toBeInTheDocument();
  expect(assetLoad).toHaveBeenCalledWith(
    "/models/heroes/runekeeper.glb", expect.any(Function), undefined, expect.any(Function),
  );
  // The cache's source model remains reusable by another preview or the Board.
  expect(scene.scale.toArray()).toEqual([1, 1, 1]);
  expect(scene.position.toArray()).toEqual([0, 0, 0]);
});

it("retains procedural geometry after a configured model fails to load", async () => {
  vi.spyOn(browserRenderer, "canCreateWebGLContext").mockReturnValue(true);
  renderer.mountScene = true;
  assetLoad.mockImplementationOnce((_path, _onLoad, _onProgress, onError) => onError(new Error("Asset missing")));
  const { container } = render(<HeroPreview3D heroType="pyromancer" label="Pyromancer" />);
  await act(async () => undefined);
  expect(assetLoad).toHaveBeenCalledWith(
    "/models/heroes/pyromancer.glb", expect.any(Function), undefined, expect.any(Function),
  );
  expect(container.querySelector("mesh")).toBeInTheDocument();
  expect(container.querySelector("primitive")).not.toBeInTheDocument();
});

it("uses procedural geometry without a download for a Hero without a configured model", () => {
  vi.spyOn(browserRenderer, "canCreateWebGLContext").mockReturnValue(true);
  renderer.mountScene = true;
  const { container } = render(<HeroPreview3D heroType="archer" label="Archer" />);
  expect(container.querySelector("mesh")).toBeInTheDocument();
  expect(container.querySelector("primitive")).not.toBeInTheDocument();
  expect(assetLoad).not.toHaveBeenCalled();
});

it("lets a resolved account 2D preference override local 3D and respond to preference changes", () => {
  const probe = vi.spyOn(browserRenderer, "canCreateWebGLContext").mockReturnValue(true);
  const { rerender } = render(<HeroPreview3D heroType="warden" label="Warden" visualMode="2d" />);
  expect(screen.getByRole("img", { name: "Warden portrait" })).toBeInTheDocument();
  expect(probe).not.toHaveBeenCalled();
  rerender(<HeroPreview3D heroType="warden" label="Warden" visualMode="3d" />);
  expect(screen.getByLabelText("Hero renderer")).toBeInTheDocument();
  rerender(<HeroPreview3D heroType="warden" label="Warden" visualMode="2d" />);
  expect(screen.getByRole("img", { name: "Warden portrait" })).toBeInTheDocument();
  expect(screen.queryByLabelText("Hero renderer")).not.toBeInTheDocument();
});
