#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
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

pub struct Info {
    pub action: Action,
    pub key: &'static str,
    pub display: &'static str,
    pub default_hotkey: &'static str,
}

/// Mirrors the C# `Actions.All` table; `key` matches the TS WindowAction names.
pub const ALL: &[Info] = &[
    Info { action: Action::LeftHalf, key: "LeftHalf", display: "Left Half", default_hotkey: "Ctrl+Alt+Left" },
    Info { action: Action::RightHalf, key: "RightHalf", display: "Right Half", default_hotkey: "Ctrl+Alt+Right" },
    Info { action: Action::TopHalf, key: "TopHalf", display: "Top Half", default_hotkey: "Ctrl+Alt+Up" },
    Info { action: Action::BottomHalf, key: "BottomHalf", display: "Bottom Half", default_hotkey: "Ctrl+Alt+Down" },
    Info { action: Action::TopLeftQuarter, key: "TopLeftQuarter", display: "Top-Left Quarter", default_hotkey: "Ctrl+Alt+U" },
    Info { action: Action::TopRightQuarter, key: "TopRightQuarter", display: "Top-Right Quarter", default_hotkey: "Ctrl+Alt+I" },
    Info { action: Action::BottomLeftQuarter, key: "BottomLeftQuarter", display: "Bottom-Left Quarter", default_hotkey: "Ctrl+Alt+J" },
    Info { action: Action::BottomRightQuarter, key: "BottomRightQuarter", display: "Bottom-Right Quarter", default_hotkey: "Ctrl+Alt+K" },
    Info { action: Action::FirstThird, key: "FirstThird", display: "First Third", default_hotkey: "Ctrl+Alt+D" },
    Info { action: Action::CenterThird, key: "CenterThird", display: "Center Third", default_hotkey: "Ctrl+Alt+F" },
    Info { action: Action::LastThird, key: "LastThird", display: "Last Third", default_hotkey: "Ctrl+Alt+G" },
    Info { action: Action::FirstTwoThirds, key: "FirstTwoThirds", display: "First Two-Thirds", default_hotkey: "Ctrl+Alt+E" },
    Info { action: Action::LastTwoThirds, key: "LastTwoThirds", display: "Last Two-Thirds", default_hotkey: "Ctrl+Alt+T" },
    Info { action: Action::Maximize, key: "Maximize", display: "Maximize", default_hotkey: "Ctrl+Alt+Enter" },
    Info { action: Action::AlmostMaximize, key: "AlmostMaximize", display: "Almost Maximize", default_hotkey: "" },
    Info { action: Action::MaximizeHeight, key: "MaximizeHeight", display: "Maximize Height", default_hotkey: "" },
    Info { action: Action::Center, key: "Center", display: "Center", default_hotkey: "Ctrl+Alt+C" },
    Info { action: Action::Restore, key: "Restore", display: "Restore", default_hotkey: "Ctrl+Alt+Back" },
    Info { action: Action::MakeLarger, key: "MakeLarger", display: "Make Larger", default_hotkey: "Ctrl+Alt+Oemplus" },
    Info { action: Action::MakeSmaller, key: "MakeSmaller", display: "Make Smaller", default_hotkey: "Ctrl+Alt+OemMinus" },
    Info { action: Action::NextDisplay, key: "NextDisplay", display: "Next Display", default_hotkey: "Ctrl+Alt+Win+Right" },
    Info { action: Action::PreviousDisplay, key: "PreviousDisplay", display: "Previous Display", default_hotkey: "Ctrl+Alt+Win+Left" },
];

impl Action {
    pub fn from_key(s: &str) -> Option<Action> {
        ALL.iter().find(|i| i.key == s).map(|i| i.action)
    }
    pub fn key(self) -> &'static str {
        ALL.iter().find(|i| i.action == self).map(|i| i.key).unwrap_or("")
    }
    pub fn display(self) -> &'static str {
        ALL.iter().find(|i| i.action == self).map(|i| i.display).unwrap_or("")
    }
}
