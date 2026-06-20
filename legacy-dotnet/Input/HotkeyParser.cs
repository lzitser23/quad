using WinRect.Interop;

namespace WinRect.Input;

/// <summary>A parsed global hotkey: Win32 modifier flags + virtual-key code.</summary>
public readonly record struct Hotkey(uint Modifiers, uint Vk, string Display);

/// <summary>
/// Parses human-readable hotkey strings like "Ctrl+Alt+Left" into Win32 modifier/VK pairs.
/// Token names are case-insensitive. An empty/blank string parses to null (action unbound).
/// </summary>
public static class HotkeyParser
{
    // Virtual-key codes (winuser.h). Letters/digits use their ASCII code.
    private static readonly Dictionary<string, uint> KeyMap = new(StringComparer.OrdinalIgnoreCase)
    {
        ["left"] = 0x25, ["up"] = 0x26, ["right"] = 0x27, ["down"] = 0x28,
        ["enter"] = 0x0D, ["return"] = 0x0D,
        ["space"] = 0x20,
        ["back"] = 0x08, ["backspace"] = 0x08,
        ["delete"] = 0x2E, ["del"] = 0x2E,
        ["insert"] = 0x2D, ["ins"] = 0x2D,
        ["home"] = 0x24, ["end"] = 0x23,
        ["pageup"] = 0x21, ["pagedown"] = 0x22,
        ["tab"] = 0x09, ["escape"] = 0x1B, ["esc"] = 0x1B,
        // OEM punctuation
        ["oemplus"] = 0xBB, ["plus"] = 0xBB, ["="] = 0xBB,
        ["oemminus"] = 0xBD, ["minus"] = 0xBD, ["-"] = 0xBD,
        ["oemcomma"] = 0xBC, [","] = 0xBC,
        ["oemperiod"] = 0xBE, ["."] = 0xBE,
        ["oem1"] = 0xBA, [";"] = 0xBA,
        ["oem2"] = 0xBF, ["/"] = 0xBF,
        ["oem4"] = 0xDB, ["["] = 0xDB,
        ["oem6"] = 0xDD, ["]"] = 0xDD,
    };

    public static bool TryParse(string? text, out Hotkey hotkey)
    {
        hotkey = default;
        if (string.IsNullOrWhiteSpace(text)) return false;

        uint mods = 0, vk = 0;
        var displayParts = new List<string>();
        bool haveKey = false;

        foreach (var raw in text.Split('+', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            switch (raw.ToLowerInvariant())
            {
                case "ctrl" or "control":
                    mods |= NativeMethods.MOD_CONTROL; displayParts.Insert(0, "Ctrl"); continue;
                case "alt" or "option" or "opt":
                    mods |= NativeMethods.MOD_ALT; displayParts.Add("Alt"); continue;
                case "shift":
                    mods |= NativeMethods.MOD_SHIFT; displayParts.Add("Shift"); continue;
                case "win" or "cmd" or "command" or "meta" or "super":
                    mods |= NativeMethods.MOD_WIN; displayParts.Add("Win"); continue;
            }

            if (haveKey) return false; // more than one non-modifier key

            if (KeyMap.TryGetValue(raw, out var mapped))
            {
                vk = mapped;
            }
            else if (raw.Length == 1 && (char.IsLetterOrDigit(raw[0])))
            {
                vk = char.ToUpperInvariant(raw[0]);
            }
            else if (raw.Length >= 2 && (raw[0] is 'f' or 'F') && int.TryParse(raw.AsSpan(1), out var fn) && fn is >= 1 and <= 24)
            {
                vk = (uint)(0x70 + (fn - 1)); // VK_F1 = 0x70
            }
            else
            {
                return false;
            }

            displayParts.Add(NormalizeKeyName(raw));
            haveKey = true;
        }

        if (!haveKey || vk == 0) return false;

        hotkey = new Hotkey(mods, vk, string.Join("+", displayParts));
        return true;
    }

    private static string NormalizeKeyName(string raw)
    {
        if (raw.Length == 1) return raw.ToUpperInvariant();
        return raw switch
        {
            _ when raw.Equals("oemplus", StringComparison.OrdinalIgnoreCase) => "+",
            _ when raw.Equals("oemminus", StringComparison.OrdinalIgnoreCase) => "-",
            _ when raw.Equals("back", StringComparison.OrdinalIgnoreCase) => "Backspace",
            _ => char.ToUpperInvariant(raw[0]) + raw[1..].ToLowerInvariant(),
        };
    }
}
