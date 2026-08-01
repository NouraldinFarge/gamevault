import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { LocalFilesPage } from "../features/files/LocalFilesPage";
import { HomePage } from "../features/home/HomePage";
import { GameDetailsPage } from "../features/library/GameDetailsPage";
import { LibraryPage } from "../features/library/LibraryPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { AppShell } from "./AppShell";

const rootRoute = createRootRoute({
  component: () => (
    <AppShell>
      <Outlet />
    </AppShell>
  ),
  notFoundComponent: () => (
    <div className="page-state">
      <p className="eyebrow">Not found</p>
      <h1>This view is no longer available.</h1>
      <p>Return to Home or Library from the navigation.</p>
    </div>
  ),
});

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: HomePage,
});

const libraryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/library",
  component: LibraryPage,
});

const gameRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/library/$gameId",
  component: GameDetailsPage,
});

const filesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/files",
  component: LocalFilesPage,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsPage,
});

const routeTree = rootRoute.addChildren([
  homeRoute,
  libraryRoute,
  gameRoute,
  filesRoute,
  settingsRoute,
]);

export const router = createRouter({
  routeTree,
  history: createHashHistory(),
  defaultPreload: "intent",
  defaultPreloadStaleTime: 30_000,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

export function AppRouter() {
  return <RouterProvider router={router} />;
}
