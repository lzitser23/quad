using System.Drawing;
using System.Drawing.Drawing2D;

namespace WinRect.UI;

/// <summary>Generates the tray icon at runtime (a split window glyph) — no .ico asset needed.</summary>
public static class AppIcon
{
    public static Icon Create()
    {
        using var bmp = new Bitmap(32, 32);
        using (var g = Graphics.FromImage(bmp))
        {
            g.SmoothingMode = SmoothingMode.AntiAlias;
            g.Clear(Color.Transparent);

            var rect = new Rectangle(3, 5, 25, 22);
            using var fill = new SolidBrush(Color.FromArgb(0, 120, 215));
            using var pen = new Pen(Color.White, 2.2f);
            g.FillRectangle(fill, rect);
            g.DrawRectangle(pen, rect);
            // vertical split → evokes "left / right halves"
            g.DrawLine(pen, 16, 6, 16, 26);
        }

        // NOTE: GetHicon allocates an HICON kept for the app's lifetime (single icon — negligible).
        return Icon.FromHandle(bmp.GetHicon());
    }
}
