import { expect, test, type Page } from "@playwright/test";

test("Wiki shows Movement Cards and entry AP before Attack and final Card Play", async ({ page }) => {
  await page.goto("/wiki/action-points");
  const next = page.getByRole("button", { name: "Next", exact: true });
  await expect(callout(page, "Hero AP")).toHaveText("3/3");
  await expect(callout(page, "Mana")).toHaveText("3/3");
  await next.click();
  await expect(page.getByRole("heading", { name: "Summon during Movement" })).toBeVisible();
  await expect(callout(page, "Phase")).toHaveText("Movement");
  await expect(callout(page, "Mana")).toHaveText("2/3");
  await expect(callout(page, "Hero AP")).toHaveText("3/3");
  await expect(callout(page, "Unit AP")).toHaveText("1/2");
  await next.click();
  await expect(page.getByRole("button", { name: "q 0, r 1, occupied by your unit" })).toBeVisible();
  await expect(callout(page, "Unit AP")).toHaveText("0/2");
  await next.click();
  await expect(callout(page, "Phase")).toHaveText("Attack");
  await expect(callout(page, "Proactive Cards")).toHaveText("Unavailable");
  await next.click();
  await expect(callout(page, "Phase")).toHaveText("Card Play");
  await next.click();
  await expect(callout(page, "Mana")).toHaveText("1/3");
  await expect(callout(page, "Hero AP")).toHaveText("3/3");
  await expect(next).toBeDisabled();
  await page.getByRole("button", { name: "Reset", exact: true }).click();
  await expect(callout(page, "Phase")).toHaveText("Movement");
  await expect(callout(page, "Mana")).toHaveText("3/3");
});

test("Wiki keeps a summon pending through a response and resumes Movement locally", async ({ page }) => {
  const matchRequests: string[] = [];
  page.on("request", (request) => {
    if (new URL(request.url()).pathname.startsWith("/api/matches")) {
      matchRequests.push(request.url());
    }
  });
  await page.goto("/wiki/cards-and-priority");
  const next = page.getByRole("button", { name: "Next", exact: true });
  const summoned = page.getByRole("button", { name: "q 0, r 0, occupied by your unit" });
  await expect(callout(page, "Phase")).toHaveText("Movement");
  await expect(callout(page, "Priority")).toHaveText("Opponent");
  await expect(summoned).toHaveCount(0);
  await next.click();
  await expect(callout(page, "Top of stack")).toHaveText("Spark Jolt");
  await expect(callout(page, "Priority")).toHaveText("Player");
  await next.click();
  await expect(callout(page, "Hero HP")).toHaveText("19/20");
  await expect(callout(page, "Stack items")).toHaveText("1");
  await expect(summoned).toHaveCount(0);
  await next.click();
  await expect(page.getByRole("heading", { name: "Movement resumes" })).toBeVisible();
  await expect(callout(page, "Phase")).toHaveText("Movement");
  await expect(callout(page, "Stack items")).toHaveText("0");
  await expect(callout(page, "Hero AP")).toHaveText("3/3");
  await expect(callout(page, "Unit AP")).toHaveText("1/2");
  await expect(summoned).toBeVisible();
  expect(matchRequests).toEqual([]);
});

function callout(page: Page, label: string) {
  return page.getByLabel("Scene callouts").getByText(label, { exact: true }).locator("..").locator("dd");
}
