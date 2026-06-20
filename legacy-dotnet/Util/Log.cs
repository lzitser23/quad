using System.Diagnostics;

namespace WinRect;

/// <summary>Minimal, crash-proof logging to %APPDATA%\WinRect\winrect.log (and the debugger).</summary>
public static class Log
{
    private static readonly object Gate = new();
    private static readonly string LogPath =
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "WinRect", "winrect.log");

    /// <summary>Absolute path of the log file (for "Open log" actions).</summary>
    public static string FilePath => LogPath;

    public static void Info(string message) => Write("INFO", message);
    public static void Warn(string message) => Write("WARN", message);
    public static void Error(string message) => Write("ERROR", message);

    private static void Write(string level, string message)
    {
        var line = $"{DateTime.Now:yyyy-MM-dd HH:mm:ss} [{level}] {message}";
        Debug.WriteLine(line);
        try
        {
            lock (Gate)
            {
                Directory.CreateDirectory(Path.GetDirectoryName(LogPath)!);
                // Keep the log from growing unbounded.
                if (File.Exists(LogPath) && new FileInfo(LogPath).Length > 256 * 1024)
                    File.WriteAllText(LogPath, "");
                File.AppendAllText(LogPath, line + Environment.NewLine);
            }
        }
        catch { /* logging must never throw */ }
    }
}
