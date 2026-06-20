using System.Windows.Forms;
using WinRect.UI;

namespace WinRect;

internal static class Program
{
    [STAThread]
    private static void Main(string[] args)
    {
        // Single instance: a second launch just exits (the first is in the tray).
        using var mutex = new Mutex(initiallyOwned: false, @"Local\WinRect_SingleInstance_v1", out _);
        bool owned;
        try { owned = mutex.WaitOne(0); }
        catch (AbandonedMutexException) { owned = true; }

        if (!owned)
        {
            MessageBox.Show("WinRect is already running — look for its icon in the system tray.",
                "WinRect", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return;
        }

        Application.SetHighDpiMode(HighDpiMode.PerMonitorV2);
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);

        bool openOnStart = args.Any(a =>
            string.Equals(a, "--open", StringComparison.OrdinalIgnoreCase) ||
            string.Equals(a, "/open", StringComparison.OrdinalIgnoreCase));

        try
        {
            Application.Run(new TrayContext(openOnStart));
        }
        catch (Exception ex)
        {
            Log.Error($"Fatal: {ex}");
            MessageBox.Show($"WinRect hit a fatal error:\n\n{ex.Message}", "WinRect",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
        finally
        {
            try { mutex.ReleaseMutex(); } catch { /* not owned */ }
        }
    }
}
