using System.Diagnostics;
using System.Net.Http;
using Microsoft.Web.WebView2.Core;

namespace WinRect;

/// <summary>Detects and (best-effort) installs the Evergreen WebView2 runtime.</summary>
public static class WebView2Runtime
{
    // Official Evergreen bootstrapper (tiny; downloads the runtime).
    private const string BootstrapperUrl = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

    public static bool IsInstalled()
    {
        try
        {
            var v = CoreWebView2Environment.GetAvailableBrowserVersionString();
            return !string.IsNullOrEmpty(v);
        }
        catch
        {
            return false;
        }
    }

    /// <summary>Downloads and runs the Evergreen bootstrapper. May raise a UAC prompt.</summary>
    public static async Task<bool> TryInstallAsync()
    {
        try
        {
            var exe = Path.Combine(Path.GetTempPath(), "MicrosoftEdgeWebview2Setup.exe");
            using (var http = new HttpClient())
            await using (var fs = File.Create(exe))
            {
                var bytes = await http.GetByteArrayAsync(BootstrapperUrl);
                await fs.WriteAsync(bytes);
            }

            var psi = new ProcessStartInfo(exe, "/silent /install") { UseShellExecute = true };
            var proc = Process.Start(psi);
            if (proc is null) return false;
            await proc.WaitForExitAsync();
            return IsInstalled();
        }
        catch (Exception ex)
        {
            Log.Error($"WebView2 runtime install failed: {ex.Message}");
            return false;
        }
    }
}
