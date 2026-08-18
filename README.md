<p align="center">
  <img src="assets/icon.svg" width="96" alt="Pasta Handler — a happy little bowl of noodles">
</p>

<h1 align="center">Pasta Handler</h1>

<p align="center">
  Press a hotkey, get your text on the clipboard, <kbd>Ctrl</kbd>+<kbd>V</kbd> anywhere.<br>
  A tiny Windows tray utility for people who paste the same things a lot.
</p>

---

## Install

**[⬇ Download the installer](https://github.com/Kwuasimoto/PastaHandler/releases/latest/download/pastahandler-setup.exe)** — run it, done. No admin rights needed (installs per-user). This link always serves the newest release.

> **"Windows protected your PC"?** That's Microsoft SmartScreen being cautious about unsigned
> software from small projects. Click **More info → Run anyway**. The installer is built from
> exactly the code in this repository; code-signing (which removes the warning) is planned.

After install you'll find two entries in the Start menu:

| Entry | What it does |
|---|---|
| **Pasta Handler** | The tray app — lives next to your clock, listens for your hotkeys |
| **Pasta Handler Settings** | The manager window — add snippets, record hotkeys, toggle them on/off |

## Using it

1. Open **Pasta Handler Settings** (or click the tray bowl → Open Settings).
2. **Add snippet** → type your text → click the hotkey button → **press your combo** (e.g. <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>1</kbd>).
3. The row lights up — the snippet is live. Edits save automatically.
4. Anywhere in Windows: press your combo, then <kbd>Ctrl</kbd>+<kbd>V</kbd>.

Extras: the toggle parks a snippet without deleting it; two snippets may share a hotkey if only
one is active (alternate texts you switch between); the config is a hand-editable TOML at
`%APPDATA%\pastahandler\config.toml`.

## Make it yours

The swatch button in the header opens the theme editor: five colors and a corner radius — every
other shade in the UI is derived from those, so any combination stays cohesive. Ten preset
palettes ship (Sakura, Cosmic, Coffee, League, Halo, …), each previewed as a card in its own
colors. Beyond colors: a **borderless mode** swaps the OS window frame for the app's own title
bar, a **background image** (bring your own PNG, or one click on the bundled sakura art), and an
**opacity** slider that turns the canvas to frosted or clear glass over your desktop — the
controls stay solid and readable. Everything applies live and saves automatically.

## Fair-play note (League of Legends and other games)

Pasta Handler never touches any game: no input simulation, no hooks, no process access — it only
writes your clipboard when *you* press your hotkey, and *you* press Ctrl+V yourself. The design
rationale against Riot's third-party-software policy is documented in [COMPLIANCE.md](COMPLIANCE.md).
One behavioral note applies to *you*, not the tool: pasting the same link into chat every game can
be reported as spam under Riot's rules. Paste in context, not on cooldown.

## Building from source

```powershell
git clone https://github.com/Kwuasimoto/PastaHandler
cd PastaHandler
cargo build --release          # → target\release\pastahandler.exe
# installer (requires Inno Setup 6):
iscc installer\pastahandler.iss
```

Dev loop: `.\scripts\dev.ps1` rebuilds and relaunches on every save; <kbd>F12</kbd> in a debug
settings window opens egui's live style editor.

## License

MIT (see LICENSE) — © Kwuasimoto.
