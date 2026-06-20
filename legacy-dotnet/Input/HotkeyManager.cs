using System.Runtime.InteropServices;
using System.Windows.Forms;
using WinRect.Config;
using WinRect.Core;
using WinRect.Interop;

namespace WinRect.Input;

/// <summary>
/// Registers global hotkeys against a hidden message window and raises <see cref="ActionRequested"/>
/// when one fires. Collects per-action registration results so the UI can report conflicts —
/// e.g. Ctrl+Alt+Arrow being taken by another app.
/// </summary>
public sealed class HotkeyManager : NativeWindow, IDisposable
{
    public sealed record Registration(WindowAction Action, string Spec, string Display, bool Success, string? Error);

    private readonly Dictionary<int, WindowAction> _idToAction = new();
    private int _nextId = 1;

    public event Action<WindowAction>? ActionRequested;
    public IReadOnlyList<Registration> Registrations { get; private set; } = Array.Empty<Registration>();

    public HotkeyManager()
    {
        // Invisible static control: a valid HWND that receives our posted WM_HOTKEY messages.
        CreateHandle(new CreateParams { ClassName = "STATIC" });
    }

    public void RegisterAll(AppSettings settings)
    {
        UnregisterAllInternal();

        var results = new List<Registration>();
        foreach (var info in Actions.All)
        {
            var spec = settings.HotkeyFor(info.Action);
            if (string.IsNullOrWhiteSpace(spec))
            {
                results.Add(new Registration(info.Action, "", "", true, null)); // intentionally unbound
                continue;
            }

            if (!HotkeyParser.TryParse(spec, out var hk))
            {
                results.Add(new Registration(info.Action, spec, spec, false, "could not parse hotkey"));
                continue;
            }

            int id = _nextId++;
            bool ok = NativeMethods.RegisterHotKey(Handle, id, hk.Modifiers | NativeMethods.MOD_NOREPEAT, hk.Vk);
            if (ok)
            {
                _idToAction[id] = info.Action;
                results.Add(new Registration(info.Action, spec, hk.Display, true, null));
            }
            else
            {
                int err = Marshal.GetLastWin32Error();
                results.Add(new Registration(info.Action, spec, hk.Display, false, Describe(err)));
                Log.Warn($"Hotkey '{hk.Display}' for {info.Action} failed: {Describe(err)}");
            }
        }

        Registrations = results;
    }

    private static string Describe(int err) => err switch
    {
        1409 => "already in use by another app or Windows",
        _ => $"win32 error {err}",
    };

    protected override void WndProc(ref Message m)
    {
        if (m.Msg == NativeMethods.WM_HOTKEY && _idToAction.TryGetValue(m.WParam.ToInt32(), out var action))
        {
            try { ActionRequested?.Invoke(action); }
            catch (Exception ex) { Log.Error($"Action {action} threw: {ex}"); }
            return;
        }
        base.WndProc(ref m);
    }

    private void UnregisterAllInternal()
    {
        foreach (var id in _idToAction.Keys)
            NativeMethods.UnregisterHotKey(Handle, id);
        _idToAction.Clear();
    }

    public void Dispose()
    {
        UnregisterAllInternal();
        if (Handle != IntPtr.Zero) DestroyHandle();
    }
}
