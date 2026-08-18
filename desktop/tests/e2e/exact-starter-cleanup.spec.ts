import { expect, test } from "@playwright/test";

import { installMockBridge } from "../helpers/bridge";

const OLD_FIZZ = "a".repeat(64);
const CURRENT_FIZZ = "b".repeat(64);
const FULLY = "c".repeat(64);

async function openAgents(page: import("@playwright/test").Page) {
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await expect(page.getByTestId("open-agents-view")).toBeVisible();
  await page.getByTestId("open-agents-view").click();
  await expect(page.getByTestId("agents-page-content")).toBeVisible();
}

test("exact starter cleanup keeps same-named identities distinct", async ({
  page,
}) => {
  await installMockBridge(page, {
    managedAgents: [
      {
        pubkey: OLD_FIZZ,
        name: "Fizz",
        personaId: "builtin:fizz",
        status: "stopped",
      },
      {
        pubkey: CURRENT_FIZZ,
        name: "Fizz",
        personaId: "builtin:fizz",
        status: "stopped",
      },
      {
        pubkey: FULLY,
        name: "Fully",
        personaId: "custom:fully",
        status: "stopped",
      },
    ],
  });
  await openAgents(page);

  await page.getByTestId("exact-starter-cleanup-button").click();
  const dialog = page.getByTestId("exact-starter-cleanup-dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText(OLD_FIZZ, { exact: true })).toBeVisible();
  await expect(dialog.getByText(CURRENT_FIZZ, { exact: true })).toBeVisible();
  await expect(dialog.getByText(FULLY, { exact: true })).toHaveCount(0);

  await page.getByTestId(`starter-cleanup-entry-${OLD_FIZZ}`).click();
  await dialog.screenshot({
    path: "test-results/exact-starter-cleanup-dialog.png",
  });
  const confirmation = page.getByTestId("exact-starter-delete-confirmation");
  const deleteButton = page.getByTestId("exact-starter-delete-button");

  await confirmation.fill(OLD_FIZZ.toUpperCase());
  await expect(deleteButton).toBeDisabled();
  await confirmation.fill(OLD_FIZZ);
  await expect(deleteButton).toBeEnabled();
  await deleteButton.click();

  await expect(
    page.getByTestId(`starter-cleanup-entry-${OLD_FIZZ}`),
  ).toHaveCount(0);
  await expect(
    page.getByTestId(`starter-cleanup-entry-${CURRENT_FIZZ}`),
  ).toBeVisible();
});

test("running starter identity cannot be deleted", async ({ page }) => {
  await installMockBridge(page, {
    managedAgents: [
      {
        pubkey: OLD_FIZZ,
        name: "Fizz",
        personaId: "builtin:fizz",
        status: "running",
      },
    ],
  });
  await openAgents(page);

  await page.getByTestId("exact-starter-cleanup-button").click();
  await page.getByTestId(`starter-cleanup-entry-${OLD_FIZZ}`).click();
  await expect(
    page.getByText("Stop this identity before deleting it."),
  ).toBeVisible();
  await expect(page.getByTestId("exact-starter-delete-button")).toBeDisabled();
});
