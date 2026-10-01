import { expect, test } from "@playwright/test";

for (const fallback of ["2d", "reduced-motion", "unavailable-webgl"] as const) {
  test(`Hero picker remains selectable with ${fallback}`, async ({ page }) => {
    const errors: string[] = [];
    const apiRequests: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("request", (request) => {
      if (new URL(request.url()).pathname.includes("/api/")) apiRequests.push(request.url());
    });
    if (fallback === "reduced-motion") {
      await page.emulateMedia({ reducedMotion: "reduce" });
    } else {
      await page.addInitScript((mode) => {
        localStorage.setItem("rune-lanes-board-visual-mode", mode);
        if (mode === "3d") {
          const original = HTMLCanvasElement.prototype.getContext;
          HTMLCanvasElement.prototype.getContext = function (...args) {
            if (String(args[0]).includes("webgl")) return null;
            return original.apply(this, args);
          };
        }
      }, fallback === "2d" ? "2d" : "3d");
    }
    await page.goto("./workshop");
    const picker = page.getByRole("group", { name: "Hero type", exact: true });
    await expect(picker.getByRole("img", { name: "Runekeeper portrait" })).toBeVisible();
    const warden = picker.getByRole("button", { name: /^Warden/ });
    await warden.focus();
    await page.keyboard.press("Enter");
    await expect(warden).toHaveAttribute("aria-pressed", "true");
    await expect(picker.getByRole("img", { name: "Warden portrait" })).toBeVisible();
    await expect(page.getByRole("combobox", { name: "Your Hero", exact: true })).toHaveValue("warden");
    await expect(page.getByRole("combobox", { name: "Bot Hero", exact: true })).toHaveValue("runekeeper");
    await page.getByRole("button", { name: "Play against bot", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Round 1", exact: true })).toBeVisible();
    expect(errors).toEqual([]);
    expect(apiRequests).toEqual([]);
  });
}

for (const failedAsset of [false, true]) {
  test(`3D Hero picker ${failedAsset ? "uses a miniature after a failed GLB" : "loads configured models and unconfigured miniatures"}`, async ({ page }, testInfo) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.addInitScript(() => localStorage.setItem("rune-lanes-board-visual-mode", "3d"));
    if (failedAsset) await page.route("**/models/heroes/runekeeper.glb", (route) => route.abort());
    const modelRequest = page.waitForRequest("**/models/heroes/runekeeper.glb");
    const modelResponse = failedAsset ? null : page.waitForResponse("**/models/heroes/runekeeper.glb");
    await page.goto("./workshop");
    await modelRequest;
    if (modelResponse) expect((await modelResponse).status()).toBe(200);
    const picker = page.getByRole("group", { name: "Hero type", exact: true });
    await expect(picker.locator("canvas")).toBeVisible();
    await expect.poll(() => picker.locator("canvas").evaluate((canvas) =>
      canvas.width > 0 && canvas.height > 0,
    )).toBe(true);
    await page.screenshot({ path: testInfo.outputPath("hero-stage.png"), fullPage: true });
    await expect(picker.getByRole("img")).toHaveCount(0);
    await picker.getByRole("button", { name: /^Archer/ }).click();
    await expect(page.getByRole("combobox", { name: "Your Hero", exact: true })).toHaveValue("archer");
    await expect(picker.locator("canvas")).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath("hero-miniature.png"), fullPage: true });
    if (!failedAsset) {
      const pyromancer = page.waitForResponse("**/models/heroes/pyromancer.glb");
      await picker.getByRole("button", { name: /^Pyromancer/ }).click();
      expect((await pyromancer).status()).toBe(200);
      await expect(picker.getByRole("button", { name: /^Pyromancer/ })).toHaveAttribute("aria-pressed", "true");
    }
    expect(errors).toEqual([]);
  });
}
