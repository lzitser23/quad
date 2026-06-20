# The hotkey-spec string is the cross-language seam

A binding is carried as a spec string (`"Ctrl+Alt+Oemplus"`). Rust owns one parser
(`hotkeys::parse_shortcut` → a `global-shortcut` `Shortcut`); the web owns one
parse/format/capture module (`web/src/lib/hotkeys.ts`). The spec string is the seam between them.

We deliberately did **not** unify the grammar into a single shared definition. Rust needs
`keyboard-types::Code`; the browser needs `KeyboardEvent.code` and a human-readable form — different
vocabularies on each side. Mirroring two small owners across one string contract is cheaper than a
shared schema. Each side has one place to change and its own tests; the token names (`Oemplus`,
`Back`, …) are the thing to keep in sync.
