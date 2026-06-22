# macOS signing & notarization (CI)

The `mac` job in `build.yml` produces an **unsigned, ad-hoc** universal DMG by default. The moment
the six repository secrets below exist, the same job automatically **Developer-ID signs and
notarizes** the DMG instead — no workflow edits needed (it keys off `APPLE_SIGNING_IDENTITY`).

Signing matters for two reasons:
- **No Gatekeeper warning** — users can open the app normally instead of right-click → Open.
- **The Accessibility grant persists across builds.** macOS ties the grant to the code signature;
  ad-hoc signatures change every build, so each new DMG forces a re-grant. A stable Developer ID
  fixes that for good.

## Secrets to add (Settings → Secrets and variables → Actions)

| Secret | What it is | How to get it |
| --- | --- | --- |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Your Name (TEAMID)` | Keychain Access → your Developer ID Application cert → copy its full name. Also: `security find-identity -v -p codesigning` |
| `APPLE_CERTIFICATE` | base64 of the exported cert `.p12` | Export the Developer ID Application cert (incl. private key) from Keychain as `.p12`, then `base64 -i cert.p12 \| pbcopy` |
| `APPLE_CERTIFICATE_PASSWORD` | password you set on the `.p12` export | — |
| `APPLE_API_ISSUER` | App Store Connect API **Issuer ID** (UUID) | App Store Connect → Users and Access → Integrations → App Store Connect API |
| `APPLE_API_KEY` | App Store Connect API **Key ID** (e.g. `ABCD1234XY`) | same page, the key's ID column |
| `APPLE_API_KEY_BASE64` | base64 of the downloaded `AuthKey_<KEYID>.p8` | generate the key (Admin/App-Manager role) on that page, download once, then `base64 -i AuthKey_XXXX.p8 \| pbcopy` |

Notarization here uses the **App Store Connect API key** method (no Apple ID / app-specific
password needed). Once all six are set, push/merge and the next build is signed + notarized.
