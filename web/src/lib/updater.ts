import { Channel, invoke } from "@tauri-apps/api/core";

import type { UpdateAsset } from "./updateCheck";

export interface UpdateProgress {
  phase: "downloading" | "verifying" | "installing";
  downloadedBytes: number;
  totalBytes: number | null;
}

/**
 * Download the update, verify its checksum, and hand it to the platform swap
 * script. On success the app exits and relaunches as the new version, so the
 * returned promise only settles on failure.
 */
export async function downloadAndInstall(
  asset: UpdateAsset,
  onProgress: (progress: UpdateProgress) => void,
): Promise<void> {
  const progress = new Channel<UpdateProgress>();
  progress.onmessage = onProgress;
  const staged = await invoke<string>("download_update", {
    downloadUrl: asset.downloadUrl,
    checksumUrl: asset.checksumUrl,
    assetName: asset.name,
    onProgress: progress,
  });
  // macOS updates ship as Quad.app.zip; the swap script wants the .app.
  const platform = await invoke<string>("update_platform");
  const updatePath =
    platform === "darwin" ? await invoke<string>("extract_app_zip", { zipPath: staged }) : staged;
  onProgress({ phase: "installing", downloadedBytes: 1, totalBytes: 1 });
  await invoke("apply_update", { updatePath });
}

/** Error left behind by a failed swap after the app closed, if any (once). */
export function takeUpdateRecoveryError(): Promise<string | null> {
  return invoke<string | null>("take_update_recovery_error").catch(() => null);
}
