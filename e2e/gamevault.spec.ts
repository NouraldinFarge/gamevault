import AxeBuilder from "@axe-core/playwright";
import { expect, type Page, test } from "@playwright/test";

async function expectAppStylesReady(page: Page) {
  await expect
    .poll(
      () =>
        page.evaluate(() => {
          const shell = document.querySelector<HTMLElement>("[data-theme]");
          const skipLink = document.querySelector<HTMLAnchorElement>('a[href="#main-content"]');

          if (!shell || !skipLink) {
            return { canvasTokenReady: false, skipLinkPosition: "" };
          }

          return {
            canvasTokenReady:
              getComputedStyle(shell).getPropertyValue("--color-canvas").trim().length > 0,
            skipLinkPosition: getComputedStyle(skipLink).position,
          };
        }),
      { message: "GameVault's production styles should be applied before accessibility analysis" },
    )
    .toEqual({ canvasTokenReady: true, skipLinkPosition: "fixed" });
}

async function expectNoSeriousAccessibilityViolations(page: Page) {
  await expectAppStylesReady(page);
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
  await expect(page.getByRole("complementary", { name: "Synthetic demonstration" })).toContainText(
    "Synthetic 0.4.0 development preview",
  );
  await expect(page.getByRole("link", { name: "Verified v0.3.5 release" })).toHaveAttribute(
    "href",
    "https://github.com/NouraldinFarge/gamevault/releases/tag/v0.3.5",
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

test("the boot color contract prevents a white unstyled canvas", async ({ page }) => {
  await page.route("**/*.css", (route) => route.abort());
  await page.goto("/");

  const skipLink = page.getByRole("link", { name: "Skip to content" });
  await expect(skipLink).toBeAttached();
  await expect(page.locator("html")).toHaveCSS("background-color", "rgb(10, 15, 29)");
  await expect(page.locator("body")).toHaveCSS("color", "rgb(236, 242, 250)");
  await expect(skipLink).toHaveCSS("color", "rgb(236, 242, 250)");
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

test("primary views stay inside the viewport", async ({ page }) => {
  for (const route of ["/", "/#/library", "/#/files", "/#/settings"]) {
    await page.goto(route);
    await expect(
      page.getByRole("complementary", { name: "Synthetic demonstration" }),
    ).toBeVisible();

    const viewport = await page.evaluate(() => ({
      clientWidth: document.documentElement.clientWidth,
      scrollWidth: document.documentElement.scrollWidth,
    }));
    expect(viewport.scrollWidth, `${route} overflows the viewport`).toBeLessThanOrEqual(
      viewport.clientWidth,
    );
  }
});

test("keyboard users can skip navigation and reach the current view", async ({ page }) => {
  await page.goto("/");
  await page.keyboard.press("Tab");
  const skip = page.getByRole("link", { name: "Skip to content" });
  await expect(skip).toBeFocused();
  await skip.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
});
