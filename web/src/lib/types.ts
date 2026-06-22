// Mirrors the C# DTOs in UI/Bridge.cs (serialized camelCase).

export interface ActionDto {
  action: string;
  display: string;
  defaultHotkey: string;
  hotkey: string;
  bound: boolean;
  registered: boolean;
  error: string | null;
}

export interface SettingsDto {
  dragSnapEnabled: boolean;
  startWithWindows: boolean;
  showSnapPreview: boolean;
  snapEdgeThresholdPx: number;
  resizeStepPx: number;
  gapPx: number;
}

export interface AppState {
  version: string;
  settings: SettingsDto;
  actions: ActionDto[];
  registeredCount: number;
  failedCount: number;
  settingsPath: string;
  logPath: string;
  accessibilityOk: boolean;
}

export interface ApplyResult {
  ok: boolean;
  message: string;
}

export type SettingsPatch = Partial<SettingsDto>;
