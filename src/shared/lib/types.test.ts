import { describe, expect, it } from "vitest";
import {
  AppUpdateCheckSchema,
  DependencyAuditSchema,
  OperationRecordSchema,
  StagedUpdatePreviewSchema,
} from "./types";

const checkedAt = "2026-08-15T12:00:00Z";

describe("native response contracts", () => {
  it("accepts evidence-rich prerequisite audit results", () => {
    const result = DependencyAuditSchema.parse({
      auditedAt: checkedAt,
      managedRoot: "C:\\GameVault\\library",
      redistFolders: 1,
      filesInspected: 1,
      installed: 1,
      missing: 0,
      suspicious: 0,
      officialSourcesReachable: true,
      reportPath: "C:\\GameVault\\library\\Reports\\dependency-audit.json",
      items: [
        {
          id: "vc-redist-x64",
          name: "Microsoft Visual C++ Redistributable",
          architecture: "x64",
          bundledPath: "Redist\\vc_redist.x64.exe",
          bundledVersion: "14.44.35211.0",
          sha256: "a".repeat(64),
          signatureStatus: "Valid",
          publisher: "Microsoft Corporation",
          installedStatus: "installed",
          installedVersion: "14.44.35211.0",
          officialSourceUrl: "https://aka.ms/vc14/vc_redist.x64.exe",
          onlineStatus: "reachable",
          recommendation: "No action is required.",
          detectedBy: "registry",
          installedEvidence: ["HKLM\\SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64"],
          confidence: "high",
          publisherMatch: "verified",
          checkedAt,
        },
      ],
    });

    expect(result.items[0].installedEvidence).toHaveLength(1);
    expect(result.items[0].publisherMatch).toBe("verified");
  });

  it("accepts a resumable operation record and rejects unknown states", () => {
    const record = {
      id: "op-1",
      kind: "archive-stage",
      label: "Stage Owned Game.zip",
      status: "interrupted",
      sourcePath: "C:\\GameVault\\library\\Inbox\\Owned Game.zip",
      targetPath: "C:\\GameVault\\library\\Staging\\Owned Game-1",
      summary: "The app closed while this operation was running.",
      errorMessage: null,
      recoveryHint: "Reopen the staged package and review it before continuing.",
      reportPath: null,
      startedAt: checkedAt,
      updatedAt: checkedAt,
      completedAt: null,
    };

    expect(OperationRecordSchema.parse(record).status).toBe("interrupted");
    expect(OperationRecordSchema.safeParse({ ...record, status: "cancelled" }).success).toBe(false);
  });

  it("requires a full SHA-256 fingerprint for an update preview", () => {
    const preview = {
      isUpdate: true,
      destinationPath: "C:\\GameVault\\library\\Games\\Owned Game",
      rollbackRoot: "C:\\GameVault\\library\\Archives\\Updates",
      addedCount: 2,
      changedCount: 1,
      removedCount: 1,
      unchangedCount: 20,
      addedSample: ["content\\new.bin"],
      changedSample: ["OwnedGame.exe"],
      removedSample: ["content\\old.bin"],
      currentSizeBytes: 100,
      proposedSizeBytes: 110,
      fingerprint: "b".repeat(64),
    };

    expect(StagedUpdatePreviewSchema.parse(preview).changedCount).toBe(1);
    expect(StagedUpdatePreviewSchema.safeParse({ ...preview, fingerprint: "short" }).success).toBe(
      false,
    );
  });

  it("accepts manual update checks and rejects malformed release links", () => {
    const update = {
      currentVersion: "0.3.5",
      latestVersion: "0.4.0",
      updateAvailable: true,
      releaseUrl: "https://github.com/NouraldinFarge/gamevault/releases/tag/v0.4.0",
      publishedAt: checkedAt,
      checkedAt,
    };

    expect(AppUpdateCheckSchema.parse(update).updateAvailable).toBe(true);
    expect(AppUpdateCheckSchema.safeParse({ ...update, releaseUrl: "not a URL" }).success).toBe(
      false,
    );
  });
});
