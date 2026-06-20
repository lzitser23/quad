namespace WinRect.Core;

/// <summary>Every window-management command WinRect can perform.</summary>
public enum WindowAction
{
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter,
    FirstThird,
    CenterThird,
    LastThird,
    FirstTwoThirds,
    LastTwoThirds,
    Maximize,
    AlmostMaximize,
    MaximizeHeight,
    Center,
    Restore,
    MakeLarger,
    MakeSmaller,
    NextDisplay,
    PreviousDisplay,
}

/// <summary>Display metadata and the default (Rectangle-faithful) Windows hotkey for each action.</summary>
/// <remarks>
/// Rectangle's macOS defaults map Control→Ctrl, Option→Alt, Command→Win.
/// Actions with an empty default hotkey are available but unbound until the user sets one.
/// </remarks>
public sealed record ActionInfo(WindowAction Action, string Display, string DefaultHotkey);

public static class Actions
{
    public static readonly IReadOnlyList<ActionInfo> All = new[]
    {
        new ActionInfo(WindowAction.LeftHalf,            "Left Half",          "Ctrl+Alt+Left"),
        new ActionInfo(WindowAction.RightHalf,           "Right Half",         "Ctrl+Alt+Right"),
        new ActionInfo(WindowAction.TopHalf,             "Top Half",           "Ctrl+Alt+Up"),
        new ActionInfo(WindowAction.BottomHalf,          "Bottom Half",        "Ctrl+Alt+Down"),
        new ActionInfo(WindowAction.TopLeftQuarter,      "Top-Left Quarter",   "Ctrl+Alt+U"),
        new ActionInfo(WindowAction.TopRightQuarter,     "Top-Right Quarter",  "Ctrl+Alt+I"),
        new ActionInfo(WindowAction.BottomLeftQuarter,   "Bottom-Left Quarter","Ctrl+Alt+J"),
        new ActionInfo(WindowAction.BottomRightQuarter,  "Bottom-Right Quarter","Ctrl+Alt+K"),
        new ActionInfo(WindowAction.FirstThird,          "First Third",        "Ctrl+Alt+D"),
        new ActionInfo(WindowAction.CenterThird,         "Center Third",       "Ctrl+Alt+F"),
        new ActionInfo(WindowAction.LastThird,           "Last Third",         "Ctrl+Alt+G"),
        new ActionInfo(WindowAction.FirstTwoThirds,      "First Two-Thirds",   "Ctrl+Alt+E"),
        new ActionInfo(WindowAction.LastTwoThirds,       "Last Two-Thirds",    "Ctrl+Alt+T"),
        new ActionInfo(WindowAction.Maximize,            "Maximize",           "Ctrl+Alt+Enter"),
        new ActionInfo(WindowAction.AlmostMaximize,      "Almost Maximize",    ""),
        new ActionInfo(WindowAction.MaximizeHeight,      "Maximize Height",    ""),
        new ActionInfo(WindowAction.Center,              "Center",             "Ctrl+Alt+C"),
        new ActionInfo(WindowAction.Restore,             "Restore",            "Ctrl+Alt+Back"),
        new ActionInfo(WindowAction.MakeLarger,          "Make Larger",        "Ctrl+Alt+Oemplus"),
        new ActionInfo(WindowAction.MakeSmaller,         "Make Smaller",       "Ctrl+Alt+OemMinus"),
        new ActionInfo(WindowAction.NextDisplay,         "Next Display",       "Ctrl+Alt+Win+Right"),
        new ActionInfo(WindowAction.PreviousDisplay,     "Previous Display",   "Ctrl+Alt+Win+Left"),
    };

    public static string Display(WindowAction a) => All.First(x => x.Action == a).Display;
}
