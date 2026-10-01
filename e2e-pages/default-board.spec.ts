import { expect, test } from "@playwright/test";

test("a fresh browser can play with the default board settings", async ({ page }, testInfo) => {
  const failures: string[] = [];
  page.on("pageerror", (error) => { failures.push(error.message); });
  page.on("response", (response) => {
    if (response.status() >= 400) { failures.push(`${response.status()} ${response.url()}`); }
  });
  await page.goto("./");
  await page.getByRole("button", { name: "Play against bot", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Round 1", exact: true })).toBeVisible();
  await expect(page.getByRole("region", { name: "Hex board", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Start Attack", exact: true }).click();
  await page.getByRole("button", { name: "Finish Attacks", exact: true }).click();
  await page.getByRole("button", { name: /Rune Bruiser/ }).first().click();
  await page.getByRole("button", { name: "q 0, r 2, empty hex", exact: true }).click();
  await expect(page.getByRole("button", { name: "q 0, r 2, occupied by your unit", exact: true })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath("default-board.png"), fullPage: true });
  expect(failures).toEqual([]);
});
