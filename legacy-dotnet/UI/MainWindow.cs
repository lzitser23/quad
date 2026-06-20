using System.Diagnostics;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Windows.Forms;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;
using WinRect.Config;
using WinRect.Interop;

namespace WinRect.UI;

/// <summary>The app's main window: hosts the React/Aceternity UI in a WebView2 control.</summary>
public sealed class MainWindow : Form
{
    private readonly Bridge _bridge;
    private readonly string _webRoot;
    private WebView2? _web;
    private Control? _fallback;

    public MainWindow(Bridge bridge, string webRoot, Icon icon)
    {
        _bridge = bridge;
        _webRoot = webRoot;

        Text = "WinRect";
        Icon = icon;
        StartPosition = FormStartPosition.CenterScreen;
        // A normal sizable window — we keep WS_THICKFRAME/WS_CAPTION (native resize, Aero Snap,
        // maximize, shadow, Win11 rounded corners) but strip the visible title bar in WndProc,
        // so the React UI can draw its own chrome.
        FormBorderStyle = FormBorderStyle.Sizable;
        ClientSize = new Size(1060, 720);
        MinimumSize = new Size(860, 580);
        BackColor = Color.FromArgb(10, 10, 12);

        _ = InitAsync();
    }

    // ---- Frameless title bar -------------------------------------------------

    protected override void WndProc(ref Message m)
    {
        // Remove the non-client title bar by making the client area cover the whole window.
        if (m.Msg == NativeMethods.WM_NCCALCSIZE && m.WParam != IntPtr.Zero)
        {
            if (WindowState == FormWindowState.Maximized)
            {
                // When maximized, inset by the frame thickness so content stays within the work
                // area (and doesn't spill over the taskbar / off-screen).
                var rc = Marshal.PtrToStructure<NativeMethods.RECT>(m.LParam);
                int fx = NativeMethods.GetSystemMetrics(NativeMethods.SM_CXFRAME) +
                         NativeMethods.GetSystemMetrics(NativeMethods.SM_CXPADDEDBORDER);
                int fy = NativeMethods.GetSystemMetrics(NativeMethods.SM_CYFRAME) +
                         NativeMethods.GetSystemMetrics(NativeMethods.SM_CXPADDEDBORDER);
                rc.Left += fx; rc.Top += fy; rc.Right -= fx; rc.Bottom -= fy;
                Marshal.StructureToPtr(rc, m.LParam, false);
            }
            m.Result = IntPtr.Zero; // client area == entire window
            return;
        }

        base.WndProc(ref m);

        if (m.Msg == NativeMethods.WM_SIZE)
            PostWindowState();
    }

    private void PostWindowState() =>
        PostEvent("windowState", new { maximized = WindowState == FormWindowState.Maximized });

    private bool TryHandleWindowCommand(string request)
    {
        try
        {
            using var doc = JsonDocument.Parse(request);
            var root = doc.RootElement;
            var method = root.TryGetProperty("method", out var mEl) ? mEl.GetString() : null;
            if (method is null || !method.StartsWith("window", StringComparison.Ordinal)) return false;

            int id = root.TryGetProperty("id", out var idEl) && idEl.ValueKind == JsonValueKind.Number ? idEl.GetInt32() : 0;
            JsonElement p = root.TryGetProperty("params", out var pe) ? pe : default;

            switch (method)
            {
                case "windowMinimize":
                    WindowState = FormWindowState.Minimized;
                    break;
                case "windowToggleMaximize":
                    WindowState = WindowState == FormWindowState.Maximized ? FormWindowState.Normal : FormWindowState.Maximized;
                    PostWindowState();
                    break;
                case "windowClose":
                    Hide();
                    break;
                case "windowDrag":
                    BeginNativeMove();
                    break;
                case "windowResize":
                    BeginNativeResize(p.ValueKind == JsonValueKind.Object && p.TryGetProperty("edge", out var ed) ? ed.GetString() : null);
                    break;
                default:
                    return false;
            }

            Post(JsonSerializer.Serialize(new { id, ok = true, result = new { } }));
            return true;
        }
        catch
        {
            return false;
        }
    }

    private void BeginNativeMove()
    {
        // If maximized, dragging should restore first (Windows does this when we hand off the move).
        NativeMethods.ReleaseCapture();
        NativeMethods.SendMessage(Handle, NativeMethods.WM_NCLBUTTONDOWN, (IntPtr)NativeMethods.HTCAPTION, IntPtr.Zero);
    }

    private void BeginNativeResize(string? edge)
    {
        if (WindowState == FormWindowState.Maximized) return;
        int ht = edge switch
        {
            "left" => NativeMethods.HTLEFT,
            "right" => NativeMethods.HTRIGHT,
            "top" => NativeMethods.HTTOP,
            "bottom" => NativeMethods.HTBOTTOM,
            "topleft" => NativeMethods.HTTOPLEFT,
            "topright" => NativeMethods.HTTOPRIGHT,
            "bottomleft" => NativeMethods.HTBOTTOMLEFT,
            "bottomright" => NativeMethods.HTBOTTOMRIGHT,
            _ => 0,
        };
        if (ht == 0) return;
        NativeMethods.ReleaseCapture();
        NativeMethods.SendMessage(Handle, NativeMethods.WM_NCLBUTTONDOWN, (IntPtr)ht, IntPtr.Zero);
    }

    private async Task InitAsync()
    {
        if (!WebView2Runtime.IsInstalled())
        {
            ShowRuntimeFallback(null);
            return;
        }

        try
        {
            _fallback?.Dispose();
            _fallback = null;

            _web = new WebView2 { Dock = DockStyle.Fill };
            Controls.Add(_web);

            var env = await CoreWebView2Environment.CreateAsync(
                browserExecutableFolder: null,
                userDataFolder: Path.Combine(AppSettings.DefaultDirectory, "webview2"));
            await _web.EnsureCoreWebView2Async(env);

            var core = _web.CoreWebView2;
            core.Settings.AreDefaultContextMenusEnabled = false;
            core.Settings.IsStatusBarEnabled = false;
            core.Settings.IsZoomControlEnabled = false;
            core.Settings.AreBrowserAcceleratorKeysEnabled = false;
            core.WebMessageReceived += OnWebMessage;
            core.NewWindowRequested += OnNewWindow;

            core.SetVirtualHostNameToFolderMapping(
                "winrect.local", _webRoot, CoreWebView2HostResourceAccessKind.Allow);
            core.Navigate("https://winrect.local/index.html");
        }
        catch (Exception ex)
        {
            Log.Error($"WebView2 init failed: {ex.Message}");
            ShowRuntimeFallback(ex.Message);
        }
    }

    private void OnWebMessage(object? sender, CoreWebView2WebMessageReceivedEventArgs e)
    {
        string request;
        try { request = e.TryGetWebMessageAsString(); }
        catch { try { request = e.WebMessageAsJson; } catch { return; } }

        // Window-chrome commands (minimize/maximize/close/drag/resize) are handled here;
        // everything else goes to the app controller bridge.
        if (TryHandleWindowCommand(request)) return;

        Post(_bridge.Dispatch(request));
    }

    private void Post(string json)
    {
        try { _web?.CoreWebView2?.PostWebMessageAsString(json); }
        catch (Exception ex) { Log.Warn($"post response failed: {ex.Message}"); }
    }

    /// <summary>Push an unsolicited state change to the UI (e.g. after a tray-menu toggle).</summary>
    public void PostEvent(string name, object? payload)
    {
        if (_web?.CoreWebView2 is null || !IsHandleCreated) return;
        var json = Bridge.Event(name, payload);
        try { BeginInvoke(() => { try { _web?.CoreWebView2?.PostWebMessageAsString(json); } catch { } }); }
        catch { /* handle not ready */ }
    }

    private static void OnNewWindow(object? sender, CoreWebView2NewWindowRequestedEventArgs e)
    {
        e.Handled = true;
        if (!string.IsNullOrEmpty(e.Uri)) OpenUrl(e.Uri);
    }

    public static void OpenUrl(string url)
    {
        try { Process.Start(new ProcessStartInfo(url) { UseShellExecute = true }); }
        catch (Exception ex) { Log.Warn($"open url failed: {ex.Message}"); }
    }

    public void ShowUI()
    {
        Show();
        if (WindowState == FormWindowState.Minimized) WindowState = FormWindowState.Normal;
        Activate();
        BringToFront();
    }

    // Closing the window only hides it; the app lives in the tray.
    protected override void OnFormClosing(FormClosingEventArgs e)
    {
        if (e.CloseReason == CloseReason.UserClosing)
        {
            e.Cancel = true;
            Hide();
            return;
        }
        base.OnFormClosing(e);
    }

    private void ShowRuntimeFallback(string? detail)
    {
        var panel = new Panel { Dock = DockStyle.Fill, BackColor = Color.FromArgb(10, 10, 12), Padding = new Padding(48) };

        var title = new Label
        {
            Text = "WebView2 runtime required",
            ForeColor = Color.White,
            Font = new Font("Segoe UI", 16f, FontStyle.Bold),
            AutoSize = true,
            Location = new Point(48, 48),
        };
        var body = new Label
        {
            Text = "WinRect's UI uses the Microsoft Edge WebView2 runtime, which isn't installed yet.\n" +
                   "Your keyboard shortcuts and drag-snapping already work — this is only for the settings window.\n\n" +
                   (detail is null ? "" : $"Details: {detail}\n\n") +
                   "Click Install to download it from Microsoft (a UAC prompt may appear).",
            ForeColor = Color.Gainsboro,
            Font = new Font("Segoe UI", 10f),
            AutoSize = true,
            MaximumSize = new Size(820, 0),
            Location = new Point(48, 96),
        };
        var install = new Button
        {
            Text = "Install WebView2 runtime",
            Location = new Point(48, 220),
            Size = new Size(220, 38),
            FlatStyle = FlatStyle.Flat,
            BackColor = Color.FromArgb(0, 120, 215),
            ForeColor = Color.White,
        };
        install.FlatAppearance.BorderSize = 0;
        install.Click += async (_, _) =>
        {
            install.Enabled = false;
            install.Text = "Installing…";
            var ok = await WebView2Runtime.TryInstallAsync();
            if (ok) await InitAsync();
            else { install.Enabled = true; install.Text = "Retry install"; }
        };

        panel.Controls.Add(install);
        panel.Controls.Add(body);
        panel.Controls.Add(title);
        Controls.Add(panel);
        _fallback = panel;
    }
}
