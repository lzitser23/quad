# Native engine in Rust under Tauri, UI in WebView2

Quad's window-management engine is native Rust (Win32 via the `windows` crate) and its UI is a
React app hosted in WebView2 by **Tauri 2**. This replaced an earlier .NET 8 / WinForms + WebView2
implementation: Tauri gives a ~6.5 MB portable exe (vs ~63 MB self-contained .NET), matches the
stack of the sibling `spoon` project, and keeps the heavy window logic in Rust where the Win32
surface is small and testable.

The trade-off accepted: WebView2 is a runtime dependency (preinstalled on Win10/11), and a plain
`cargo build` runs in "dev" mode (loads `devUrl`) — production builds must go through the Tauri CLI
(`npm run tauri build`) so the frontend is embedded.
