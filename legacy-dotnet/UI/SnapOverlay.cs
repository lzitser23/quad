using System.Drawing;
using System.Drawing.Drawing2D;
using System.Windows.Forms;
using WinRect.Interop;

namespace WinRect.UI;

/// <summary>
/// A borderless, click-through, topmost translucent rectangle used to preview where a dragged
/// window will snap. Positioned with SetWindowPos in physical pixels to stay correct across
/// monitors with different DPI scaling.
/// </summary>
public sealed class SnapOverlay : Form
{
    public SnapOverlay()
    {
        FormBorderStyle = FormBorderStyle.None;
        ShowInTaskbar = false;
        StartPosition = FormStartPosition.Manual;
        TopMost = true;
        Enabled = false;
        DoubleBuffered = true;
        BackColor = Color.FromArgb(0, 120, 215); // Windows accent blue
        Opacity = 0d;
        Visible = false;
    }

    protected override bool ShowWithoutActivation => true;

    protected override CreateParams CreateParams
    {
        get
        {
            const int WS_EX_TRANSPARENT = 0x20;   // click-through
            var cp = base.CreateParams;
            cp.ExStyle |= WS_EX_TRANSPARENT
                        | (int)NativeMethods.WS_EX_TOOLWINDOW   // no taskbar/alt-tab entry
                        | (int)NativeMethods.WS_EX_NOACTIVATE;  // never steal focus
            return cp;
        }
    }

    public void ShowAt(Rectangle r)
    {
        _ = Handle; // force creation
        NativeMethods.SetWindowPos(Handle, NativeMethods.HWND_TOPMOST, r.X, r.Y, r.Width, r.Height,
            NativeMethods.SWP_NOACTIVATE | NativeMethods.SWP_SHOWWINDOW);
        Opacity = 0.35d;
        Visible = true;
        Invalidate();
    }

    public void HideOverlay()
    {
        Opacity = 0d;
        Visible = false;
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        e.Graphics.SmoothingMode = SmoothingMode.AntiAlias;
        var inner = new Rectangle(1, 1, Math.Max(0, ClientSize.Width - 3), Math.Max(0, ClientSize.Height - 3));
        using var pen = new Pen(Color.White, 2f);
        e.Graphics.DrawRectangle(pen, inner);
    }
}
