using System.IO.Compression;
using System.Reflection;
using System.Windows.Forms;
using WinRect.Config;

namespace WinRect;

/// <summary>
/// Makes the embedded React UI available on disk for WebView2's virtual-host mapping.
/// In a published build the UI is embedded as WinRect.WebAssets.zip and extracted once to
/// %LOCALAPPDATA%\WinRect\web\&lt;version&gt;. In dev builds (no embedded zip) it falls back to
/// the freshly-built web/dist folder.
/// </summary>
public static class WebAssets
{
    public static string EnsureExtracted()
    {
        var version = Application.ProductVersion is { Length: > 0 } v ? Sanitize(v) : "dev";
        var target = Path.Combine(AppSettings.DefaultDirectory, "web", version);
        var indexPath = Path.Combine(target, "index.html");

        var asm = typeof(WebAssets).Assembly;
        var resName = asm.GetManifestResourceNames().FirstOrDefault(n => n.EndsWith("WebAssets.zip", StringComparison.OrdinalIgnoreCase));

        if (resName is not null)
        {
            // Always refresh from the embedded bundle so the UI can never go stale relative to the
            // exe (the zip is tiny, so this costs only milliseconds at startup).
            try { if (Directory.Exists(target)) Directory.Delete(target, recursive: true); }
            catch (Exception ex) { Log.Warn($"Could not clear old web assets: {ex.Message}"); }

            try
            {
                Directory.CreateDirectory(target);
                using var s = asm.GetManifestResourceStream(resName)!;
                using var zip = new ZipArchive(s, ZipArchiveMode.Read);
                zip.ExtractToDirectory(target, overwriteFiles: true);
            }
            catch (Exception ex)
            {
                Log.Error($"Extracting web assets failed: {ex.Message}");
            }

            if (File.Exists(indexPath)) return target;
        }

        // Dev fallback: look for an already-built web/dist next to the binaries or in the source tree.
        foreach (var cand in DevCandidates())
        {
            if (File.Exists(Path.Combine(cand, "index.html"))) return cand;
        }

        // Last resort: a minimal placeholder so the window still opens with a useful message.
        Directory.CreateDirectory(target);
        File.WriteAllText(indexPath,
            "<!doctype html><html><body style=\"font-family:Segoe UI,sans-serif;background:#0a0a0c;color:#e5e7eb;padding:40px\">" +
            "<h2>WinRect</h2><p>Web UI assets were not found. Run <code>build.ps1</code> to build the React UI.</p></body></html>");
        return target;
    }

    private static IEnumerable<string> DevCandidates()
    {
        var baseDir = AppContext.BaseDirectory;
        yield return Path.Combine(baseDir, "wwwroot");
        // walk up from the binaries to find the project's web/dist
        var dir = new DirectoryInfo(baseDir);
        for (int i = 0; i < 6 && dir is not null; i++, dir = dir.Parent)
        {
            yield return Path.Combine(dir.FullName, "web", "dist");
        }
    }

    private static string Sanitize(string s)
    {
        foreach (var c in Path.GetInvalidFileNameChars()) s = s.Replace(c, '_');
        return s;
    }
}
