using System.Drawing;
using System.Windows.Forms;
using WinRect.Config;
using WinRect.Core;
using WinRect.Input;
using WinRect.Interop;

namespace WinRect.UI;

/// <summary>
/// Owns the whole running app: settings, the window manager, global hotkeys, drag-snapping,
/// the active-window tracker, the tray icon, and the WebView2 main window. Also implements
/// <see cref="IAppController"/> so the React UI can drive it.
/// </summary>
public sealed class TrayContext : ApplicationContext, IAppController
{
    private readonly AppSettings _settings;
    private readonly WindowManager _wm;
    private readonly HotkeyManager _hotkeys;
    private readonly DragSnapManager _drag;
    private readonly ActiveWindowTracker _tracker;
    private readonly NotifyIcon _tray;
    private readonly Icon _icon;
    private readonly string _webRoot;
    private MainWindow? _main;

    private ToolStripMenuItem _dragItem = null!;
    private ToolStripMenuItem _autostartItem = null!;
    private bool _disposed;

    public TrayContext(bool openOnStart = false)
    {
        _settings = AppSettings.Load();
        _settings.ApplyAutostart();

        _wm = new WindowManager(_settings);
        _tracker = new ActiveWindowTracker();

        _hotkeys = new HotkeyManager();
        _hotkeys.ActionRequested += action => _wm.Execute(action);
        _hotkeys.RegisterAll(_settings);

        _drag = new DragSnapManager(_wm, _settings);
        _drag.SetEnabled(_settings.DragSnapEnabled);

        _webRoot = WebAssets.EnsureExtracted();

        _icon = AppIcon.Create();
        _tray = new NotifyIcon
        {
            Icon = _icon,
            Visible = true,
            Text = "WinRect — window tiling",
            ContextMenuStrip = BuildMenu(),
        };
        _tray.DoubleClick += (_, _) => ShowMain();

        Log.Info($"Started. {_hotkeys.Registrations.Count(r => r.Success)} hotkeys registered, " +
                 $"drag-snap {(_settings.DragSnapEnabled ? "on" : "off")}.");
        ReportHotkeyFailures();

        if (openOnStart)
        {
            // Defer until the message loop is pumping so the WebView2 async init has a context.
            var t = new System.Windows.Forms.Timer { Interval = 120 };
            t.Tick += (_, _) => { t.Stop(); t.Dispose(); ShowMain(); };
            t.Start();
        }
    }

    // ---- Tray menu -----------------------------------------------------------

    private ContextMenuStrip BuildMenu()
    {
        var menu = new ContextMenuStrip();

        menu.Items.Add(new ToolStripMenuItem("Open WinRect", null, (_, _) => ShowMain())
        {
            Font = new Font(menu.Font, FontStyle.Bold),
        });
        menu.Items.Add(new ToolStripSeparator());

        _dragItem = new ToolStripMenuItem("Drag-to-snap", null, (_, _) => ToggleDragSnap())
        {
            CheckOnClick = true,
            Checked = _settings.DragSnapEnabled,
        };
        menu.Items.Add(_dragItem);

        _autostartItem = new ToolStripMenuItem("Start with Windows", null, (_, _) => ToggleAutostart())
        {
            CheckOnClick = true,
            Checked = _settings.StartWithWindows,
        };
        menu.Items.Add(_autostartItem);

        menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add(new ToolStripMenuItem("Reload settings file", null, (_, _) => ReloadSettings()));
        menu.Items.Add(new ToolStripMenuItem("Open settings file", null, (_, _) => OpenSettingsFile()));
        menu.Items.Add(new ToolStripMenuItem("Open log", null, (_, _) => OpenLog()));
        menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add(new ToolStripMenuItem("Exit WinRect", null, (_, _) => ExitApp()));

        menu.Opening += (_, _) => UpdateTrayChecks();
        return menu;
    }

    private void ToggleDragSnap()
    {
        _settings.DragSnapEnabled = _dragItem.Checked;
        _drag.SetEnabled(_settings.DragSnapEnabled);
        _settings.Save();
        PushState();
    }

    private void ToggleAutostart()
    {
        _settings.StartWithWindows = _autostartItem.Checked;
        _settings.ApplyAutostart();
        _settings.Save();
        PushState();
    }

    private void UpdateTrayChecks()
    {
        if (_dragItem is not null) _dragItem.Checked = _settings.DragSnapEnabled;
        if (_autostartItem is not null) _autostartItem.Checked = _settings.StartWithWindows;
    }

    private void ShowMain()
    {
        if (_main is null || _main.IsDisposed)
            _main = new MainWindow(new Bridge(this), _webRoot, _icon);
        _main.ShowUI();
    }

    private void ReloadSettings()
    {
        var fresh = AppSettings.Load();
        _settings.DragSnapEnabled = fresh.DragSnapEnabled;
        _settings.StartWithWindows = fresh.StartWithWindows;
        _settings.ShowSnapPreview = fresh.ShowSnapPreview;
        _settings.SnapEdgeThresholdPx = fresh.SnapEdgeThresholdPx;
        _settings.ResizeStepPx = fresh.ResizeStepPx;
        _settings.GapPx = fresh.GapPx;
        _settings.Hotkeys = fresh.Hotkeys;

        _hotkeys.RegisterAll(_settings);
        _drag.SetEnabled(_settings.DragSnapEnabled);
        _settings.ApplyAutostart();
        UpdateTrayChecks();
        PushState();
        Log.Info("Settings reloaded from disk.");
    }

    private void ReportHotkeyFailures()
    {
        var failed = _hotkeys.Registrations
            .Where(r => !string.IsNullOrWhiteSpace(r.Spec) && !r.Success)
            .ToList();
        if (failed.Count == 0) return;

        var sample = string.Join(", ", failed.Take(4).Select(f => $"{Actions.Display(f.Action)} ({f.Display})"));
        var more = failed.Count > 4 ? $" +{failed.Count - 4} more" : "";
        _tray.BalloonTipTitle = "Some shortcuts couldn't be registered";
        _tray.BalloonTipText = $"{sample}{more}. Ctrl+Alt+Arrow is often taken by Intel graphics — open WinRect to rebind.";
        _tray.ShowBalloonTip(8000);
    }

    private void PushState() => _main?.PostEvent("state", GetState());

    // ---- IAppController (called from the React UI) ---------------------------

    public AppState GetState()
    {
        var regByAction = _hotkeys.Registrations.ToDictionary(r => r.Action, r => r);
        var actions = Actions.All.Select(info =>
        {
            var spec = _settings.HotkeyFor(info.Action);
            bool bound = !string.IsNullOrWhiteSpace(spec);
            regByAction.TryGetValue(info.Action, out var reg);
            bool registered = bound && (reg?.Success ?? false);
            string? error = bound && reg is { Success: false } ? reg.Error : null;
            return new ActionDto(info.Action.ToString(), info.Display, info.DefaultHotkey, spec, bound, registered, error);
        }).ToList();

        int registeredCount = actions.Count(a => a.Registered);
        int failedCount = actions.Count(a => a.Bound && !a.Registered);

        var dto = new SettingsDto(
            _settings.DragSnapEnabled, _settings.StartWithWindows, _settings.ShowSnapPreview,
            _settings.SnapEdgeThresholdPx, _settings.ResizeStepPx, _settings.GapPx);

        return new AppState(Application.ProductVersion ?? "0", dto, actions,
            registeredCount, failedCount, _settings.FilePath, Log.FilePath);
    }

    public AppState UpdateSettings(SettingsPatch p)
    {
        if (p is null) return GetState();
        if (p.DragSnapEnabled is bool d) { _settings.DragSnapEnabled = d; _drag.SetEnabled(d); }
        if (p.StartWithWindows is bool sw) { _settings.StartWithWindows = sw; _settings.ApplyAutostart(); }
        if (p.ShowSnapPreview is bool sp) _settings.ShowSnapPreview = sp;
        if (p.SnapEdgeThresholdPx is int se) _settings.SnapEdgeThresholdPx = Math.Clamp(se, 2, 100);
        if (p.ResizeStepPx is int rs) _settings.ResizeStepPx = Math.Clamp(rs, 5, 400);
        if (p.GapPx is int g) _settings.GapPx = Math.Clamp(g, 0, 100);
        _settings.Save();
        UpdateTrayChecks();
        return GetState();
    }

    public AppState SetHotkey(string action, string spec)
    {
        if (Enum.TryParse<WindowAction>(action, ignoreCase: true, out var a))
        {
            spec = (spec ?? "").Trim();
            if (spec.Length > 0 && !HotkeyParser.TryParse(spec, out _))
                throw new InvalidOperationException($"'{spec}' is not a valid hotkey");
            _settings.Hotkeys[a.ToString()] = spec;
            _settings.Save();
            _hotkeys.RegisterAll(_settings);
        }
        return GetState();
    }

    public ApplyResult ApplyAction(string action)
    {
        if (!Enum.TryParse<WindowAction>(action, ignoreCase: true, out var a))
            return new ApplyResult(false, "Unknown action.");

        var target = _tracker.LastActive;
        if (target == IntPtr.Zero || !WindowManager.IsManageable(target))
            return new ApplyResult(false, "No recent window. Click a normal app window, then try again.");

        _wm.ExecuteOn(a, target);
        NativeMethods.SetForegroundWindow(target);
        return new ApplyResult(true, $"Applied {Actions.Display(a)}");
    }

    public void OpenLog() => MainWindow.OpenUrl(Log.FilePath);
    public void OpenSettingsFile() => MainWindow.OpenUrl(_settings.FilePath);
    public void Quit() => ExitApp();

    // ---- Lifecycle -----------------------------------------------------------

    private void ExitApp()
    {
        try { _tray.Visible = false; } catch { /* ignore */ }
        ExitThread();
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing && !_disposed)
        {
            _disposed = true;
            try { _tray.Visible = false; _tray.Dispose(); } catch { }
            _hotkeys.Dispose();
            _drag.Dispose();
            _tracker.Dispose();
            _main?.Dispose();
            _icon.Dispose();
        }
        base.Dispose(disposing);
    }
}
