import AxeBuilder from "@axe-core/playwright";
import { expect, type Page, test } from "@playwright/test";

async function expectNoSeriousAccessibilityViolations(page: Page) {
  const results = await new AxeBuilder({ page }).analyze();
  const violations = results.violations.filter(
    (violation) => violation.impact === "serious" || violation.impact === "critical",
  );
  expect(
    violations,
    violations
      .map(
        (violation) =>
          `${violation.id}: ${violation.help}\n${violation.nodes
            .map((node) => `  ${node.target.join(" ")}: ${node.failureSummary}`)
            .join("\n")}`,
      )
      .join("\n\n"),
  ).toEqual([]);
}

test("the synthetic boundary and primary journey are clear", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("complementary", { name: "Synthetic demonstration" })).toContainText(
    "No filesystem access",
  );
  await expect(page.getByRole("heading", { name: "Good to see you." })).toBeVisible();

  await page.getByRole("link", { name: "Library", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Library" })).toBeVisible();
  await page.getByPlaceholder("Search titles or tags").fill("Signal Coast");
  await expect(page.getByText("Showing 1 of 1")).toBeVisible();
  await page.getByRole("button", { name: "List layout" }).click();
  await expect(page.getByRole("button", { name: "List layout" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});

test("archive intake requires a current preview before organization", async ({ page }) => {
  await page.goto("/#/files");
  await expect(page.getByRole("heading", { name: "Local files" })).toBeVisible();

  await page.getByRole("button", { name: "Choose ZIP" }).click();
  await expect(page.getByText("ZIP verified:")).toBeVisible();
  await page.getByRole("button", { name: "Extract & analyze" }).click();
  await expect(page.getByRole("heading", { name: "Cleanup and installation plan" })).toBeVisible();

  const organize = page.getByRole("button", { name: "Organize & add game" });
  await expect(organize).toBeDisabled();
  await page.getByRole("button", { name: /Preview file changes/ }).click();
  await expect(page.getByRole("heading", { name: "File change checkpoint" })).toBeVisible();
  await expect(organize).toBeEnabled();

  await page.getByRole("button", { name: "Audit dependencies" }).click();
  await page.getByText("Evidence and version details").first().click();
  await expect(page.getByText("Classification confidence").first()).toBeVisible();
  await expect(page.getByRole("heading", { name: "Operation history" })).toBeVisible();
});

test("application updates remain an explicit review action", async ({ page }) => {
  await page.goto("/#/settings");
  await expect(page.getByRole("heading", { name: "Application updates" })).toBeVisible();
  await expect(
    page.getByText(/never downloads or installs an application update automatically/i),
  ).toBeVisible();

  await page.getByRole("button", { name: "Check for updates" }).click();
  await expect(page.getByText("Installed", { exact: true })).toBeVisible();
  await expect(page.getByText("Latest stable")).toBeVisible();
});

for (const route of ["/", "/#/library", "/#/files", "/#/settings"]) {
  test(`has no serious automated accessibility violations at ${route}`, async ({ page }) => {
    await page.goto(route);
    await expect(
      page.getByRole("complementary", { name: "Synthetic demonstration" }),
    ).toBeVisible();
    await expectNoSeriousAccessibilityViolations(page);
  });
}

test("keyboard users can skip navigation and reach the current view", async ({ page }) => {
  await page.goto("/");
  await page.keyboard.press("Tab");
  const skip = page.getByRole("link", { name: "Skip to content" });
  await expect(skip).toBeFocused();
  await skip.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
});
