import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { isDesktopRuntime, nativeClient } from "../shared/lib/native-client";
import { type ScanProgress, ScanProgressSchema } from "../shared/lib/types";

export const snapshotKey = ["application", "snapshot"] as const;

export const useSnapshot = () =>
  useQuery({
    queryKey: snapshotKey,
    queryFn: nativeClient.getSnapshot,
    staleTime: 20_000,
  });

export const useNativeEvents = () => {
  const queryClient = useQueryClient();
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);

  useEffect(() => {
    if (!isDesktopRuntime()) return;
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    void listen("library-changed", () => {
      void queryClient.invalidateQueries({ queryKey: snapshotKey });
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });
    void listen<unknown>("scan-progress", (event) => {
      const parsed = ScanProgressSchema.safeParse(event.payload);
      if (parsed.success) setScanProgress(parsed.data);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });
    return () => {
      disposed = true;
      for (const unlisten of unlisteners) unlisten();
    };
  }, [queryClient]);

  return { scanProgress, setScanProgress };
};
