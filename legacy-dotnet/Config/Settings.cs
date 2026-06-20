using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Win32;
using WinRect.Core;

namespace WinRect.Config;

/// <summary>User-editable configuration, persisted to %APPDATA%\WinRect\settings.json.</summary>
public sealed class AppSettings
{
    /// <summary>Enable drag-window-to-screen-edge snapping.</summary>
    public bool DragSnapEnabled { get; set; } = true;

    /// <summary>Launch WinRect automatically on sign-in.</summary>
    public bool StartWithWindows { get; set; } = false;

    /// <summary>Show the translucent preview overlay while drag-snapping.</summary>
    public bool ShowSnapPreview { get; set; } = true;

    /// <summary>Distance (px) from a screen edge at which a drag triggers a snap.</summary>
    public int SnapEdgeThresholdPx { get; set; } = 20;

    /// <summary>Pixels added/removed per Make Larger / Make Smaller press.</summary>
    public int ResizeStepPx { get; set; } = 30;

    /// <summary>Gap (px) left between tiled windows and screen edges (0 = flush, Rectangle default).</summary>
    public int GapPx { get; set; } = 0;

    /// <summary>action name (WindowAction enum value) → hotkey string ("" = unbound).</summary>
    public Dictionary<string, string> Hotkeys { get; set; } = new();

    [JsonIgnore] public string FilePath { get; private set; } = "";

    private static readonly JsonSerializerOptions JsonOpts = new()
    {
        WriteIndented = true,
        PropertyNameCaseInsensitive = true,
        Converters = { new JsonStringEnumConverter() },
    };

    public static string DefaultDirectory =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "WinRect");

    public static AppSettings Load()
    {
        var dir = DefaultDirectory;
        Directory.CreateDirectory(dir);
        var path = Path.Combine(dir, "settings.json");

        AppSettings settings;
        if (File.Exists(path))
        {
            try
            {
                settings = JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(path), JsonOpts) ?? new AppSettings();
            }
            catch (Exception ex)
            {
                // Corrupt file: keep a backup and fall back to defaults so we never crash on start.
                try { File.Copy(path, path + ".bad", overwrite: true); } catch { /* best effort */ }
                Log.Warn($"settings.json was unreadable ({ex.Message}); reverted to defaults.");
                settings = new AppSettings();
            }
        }
        else
        {
            settings = new AppSettings();
        }

        settings.FilePath = path;
        settings.FillMissingHotkeyDefaults();
        // Persist back so a fresh/upgraded file always lists every action for easy editing.
        settings.Save();
        return settings;
    }

    /// <summary>Adds default bindings for any action missing from the file (e.g. after an upgrade).</summary>
    public void FillMissingHotkeyDefaults()
    {
        foreach (var info in Actions.All)
        {
            if (!Hotkeys.ContainsKey(info.Action.ToString()))
                Hotkeys[info.Action.ToString()] = info.DefaultHotkey;
        }
    }

    public string HotkeyFor(WindowAction action) =>
        Hotkeys.TryGetValue(action.ToString(), out var s) ? s : "";

    public void Save()
    {
        try
        {
            File.WriteAllText(FilePath, JsonSerializer.Serialize(this, JsonOpts));
        }
        catch (Exception ex)
        {
            Log.Warn($"Could not save settings: {ex.Message}");
        }
    }

    // ---- Autostart (HKCU Run key — no admin required) ------------------------

    private const string RunKey = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string RunValueName = "WinRect";

    public void ApplyAutostart()
    {
        try
        {
            using var key = Registry.CurrentUser.OpenSubKey(RunKey, writable: true)
                            ?? Registry.CurrentUser.CreateSubKey(RunKey);
            if (key is null) return;

            if (StartWithWindows)
            {
                var exe = Environment.ProcessPath;
                if (!string.IsNullOrEmpty(exe))
                    key.SetValue(RunValueName, $"\"{exe}\"");
            }
            else
            {
                if (key.GetValue(RunValueName) is not null) key.DeleteValue(RunValueName, throwOnMissingValue: false);
            }
        }
        catch (Exception ex)
        {
            Log.Warn($"Could not update autostart: {ex.Message}");
        }
    }
}
