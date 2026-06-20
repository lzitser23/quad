using System.Text.Json;

namespace WinRect.UI;

// ---- Data shapes shared with the React UI (serialized camelCase) -------------

public sealed record ActionDto(
    string Action, string Display, string DefaultHotkey, string Hotkey,
    bool Bound, bool Registered, string? Error);

public sealed record SettingsDto(
    bool DragSnapEnabled, bool StartWithWindows, bool ShowSnapPreview,
    int SnapEdgeThresholdPx, int ResizeStepPx, int GapPx);

public sealed record AppState(
    string Version, SettingsDto Settings, IReadOnlyList<ActionDto> Actions,
    int RegisteredCount, int FailedCount, string SettingsPath, string LogPath);

public sealed record ApplyResult(bool Ok, string Message);

public sealed record SettingsPatch(
    bool? DragSnapEnabled, bool? StartWithWindows, bool? ShowSnapPreview,
    int? SnapEdgeThresholdPx, int? ResizeStepPx, int? GapPx);

/// <summary>Everything the React UI can ask the native app to do.</summary>
public interface IAppController
{
    AppState GetState();
    AppState UpdateSettings(SettingsPatch patch);
    AppState SetHotkey(string action, string spec);
    ApplyResult ApplyAction(string action);
    void OpenLog();
    void OpenSettingsFile();
    void Quit();
}

/// <summary>
/// Translates JSON-RPC-ish messages from the WebView2 page into <see cref="IAppController"/> calls.
/// Request:  {"id":N,"method":"...","params":{...}}.
/// Response: {"id":N,"ok":true,"result":...}  or  {"id":N,"ok":false,"error":"..."}.
/// </summary>
public sealed class Bridge
{
    private readonly IAppController _app;
    private static readonly JsonSerializerOptions Json = new(JsonSerializerDefaults.Web);

    public Bridge(IAppController app) => _app = app;

    public string Dispatch(string requestJson)
    {
        int id = 0;
        try
        {
            using var doc = JsonDocument.Parse(requestJson);
            var root = doc.RootElement;
            if (root.TryGetProperty("id", out var idEl) && idEl.ValueKind == JsonValueKind.Number)
                id = idEl.GetInt32();

            var method = root.GetProperty("method").GetString() ?? "";
            JsonElement p = root.TryGetProperty("params", out var pe) ? pe : default;

            object? result = method switch
            {
                "getState" => _app.GetState(),
                "updateSettings" => _app.UpdateSettings(DeserializeOrDefault<SettingsPatch>(p)),
                "setHotkey" => _app.SetHotkey(Str(p, "action"), Str(p, "spec")),
                "applyAction" => _app.ApplyAction(Str(p, "action")),
                "openLog" => Run(_app.OpenLog),
                "openSettingsFile" => Run(_app.OpenSettingsFile),
                "quit" => Run(_app.Quit),
                _ => throw new InvalidOperationException($"unknown method '{method}'"),
            };

            return JsonSerializer.Serialize(new { id, ok = true, result }, Json);
        }
        catch (Exception ex)
        {
            Log.Warn($"bridge: {ex.Message}");
            return JsonSerializer.Serialize(new { id, ok = false, error = ex.Message }, Json);
        }
    }

    /// <summary>Builds an unsolicited event envelope pushed from native → JS.</summary>
    public static string Event(string name, object? payload) =>
        JsonSerializer.Serialize(new { @event = name, payload }, Json);

    private static object? Run(Action a) { a(); return new { }; }

    private static string Str(JsonElement p, string name) =>
        p.ValueKind == JsonValueKind.Object && p.TryGetProperty(name, out var v) ? v.GetString() ?? "" : "";

    private static T DeserializeOrDefault<T>(JsonElement p) where T : class =>
        (p.ValueKind is JsonValueKind.Object ? p.Deserialize<T>(Json) : JsonSerializer.Deserialize<T>("{}", Json))!;
}
