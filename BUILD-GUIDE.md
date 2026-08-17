# PastaHandler — Build Guide

**Shared keyboard:** the human writes the new/learning-heavy code; Claude takes refactors and
mechanical chunks on explicit hand-off. Default remains human-first — the
point is still relearning Rust. This guide has been reduced to the *remaining* work; completed
segments live in git history now. Companion doc: [COMPLIANCE.md](COMPLIANCE.md) — its rules are load-bearing: no
`SetWindowsHookEx`, no `SendInput`/`keybd_event`, no `OpenProcess`/`ReadProcessMemory`, no
input-simulation crates, no auto-paste. Ever.

## Where the project stands (verified 2026-08)

**Architecture (immovable): two processes, one exe.**

- **Resident** (`pastahandler.exe`) — tao loop + hotkeys + tray + clipboard. No window, no GPU
  context, 1 s `WaitUntil` heartbeat, ~0% idle CPU.
- **Settings** (`pastahandler.exe --settings`) — currently opens Notepad; becomes the eframe UI in
  the next segment. Edits the TOML; the resident notices the mtime change and re-registers live.

**Module map (all compile clean, clippy silent):**

| File | Owns |
|---|---|
| `main.rs` | dispatch only: `--settings` → `settings::run`, else `resident::run` |
| `resident.rs` | the resident process: event loop, heartbeat reload, hotkey/menu handling |
| `settings.rs` | `run(ConfigFile)` — Notepad stub, replaced by eframe next segment (same signature) |
| `config.rs` | `ConfigFile` (read / atomic write / `mtime()`), `Config`, `Snippet`, sample seeding |
| `hotkeys.rs` | `Hotkeys` with idempotent 3-pass `register_all` (validate → unregister → register) |
| `clipboard.rs` | `ClipboardManager` with 3×/50 ms retry |
| `tray.rs` | `Tray`: embedded farfalle icon (`assets/icon.png`, source `assets/icon.svg`), menu, ids |
| `error.rs` | `AppError` + `From` impls + `Display`; `Result<T>` alias |

**Already proven by test:** startup rejects a bad config with a readable error (exit 1, no panic);
a corrupt save *while running* logs `hotkey re-register failed: …` and keeps the old hotkeys live;
a good save reloads silently without restart. Clipboard contents survive app exit.

**Status update (night shift, 2026-08-16):** Segment 4 is **implemented and machine-verified** —
UI polish (sized fields, hints, subtitle), commit-and-write auto-save, shared validation via
`hotkeys::parse_all` (unit-tested ×3), config round-trip tests (×2), clippy silent,
`windows_subsystem` enabled, Notepad stub deleted. Auto-save proven end-to-end with synthetic
input: typed into the real window, tabbed away, watched `label = "NightShift"` land in the TOML.

- [x] `#![windows_subsystem = "windows"]` enabled (comment out for `println!` debugging).
- [x] Checkpoint commit.
- [ ] **The one human-required test:** hotkey → `Ctrl+V` pastes (machine can't press your keys in
  good conscience); plus the full loop — tray → Open Settings → edit → paste reflects the edit
  within a second → Quit. Then straight to Segment 5.

---

## Segment 4 — Settings UI (eframe) · 2–3 evenings

**Goal:** replace `settings::run`'s Notepad body with a small egui window: snippet table,
add/edit/delete, validated hotkey field. **Decided UX: no Save button** — every *committed* edit
writes the file; the resident picks it up within a second.

Because settings is its own process, this is a bog-standard eframe app — no event-loop integration,
no hide-to-tray. Closing the window = process exits = GPU context released. That's the whole
lifecycle.

> Each step below has a **spoiler** — a full reference implementation to type from (repetition
> learning). Try the signatures first; open the spoiler when you want the answer key. Written
> against eframe 0.36 — if a signature has drifted, the docs.rs/eframe front page wins.

### 4.0 Warm-up — the launch toggle (~20 min)

The config header comment is already in ✓. The remaining accepted feature: `open_settings_on_launch`.

<details>
<summary>Spoiler: reference implementation</summary>

```rust
// config.rs — add the field to Config (serde(default) keeps old files parsing):
#[derive(Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub snippets: Vec<Snippet>,
    #[serde(default)]
    pub open_settings_on_launch: bool,
}

// resident.rs — extract the spawn (the menu handler becomes call site #1):
fn spawn_settings() {
    match std::env::current_exe() {
        Ok(exe) => {
            if let Err(e) = std::process::Command::new(exe).arg("--settings").spawn() {
                eprintln!("failed to launch settings: {e}");
            }
        }
        Err(e) => eprintln!("current_exe failed: {e}"),
    }
}

// resident.rs, in run(), after `let tray = Tray::new()?;` — call site #2, startup-only
// (deliberately NOT re-checked in the reload path: no window popping mid-game):
if config.open_settings_on_launch {
    spawn_settings();
}

// ...and the menu handler's open_id arm collapses to: spawn_settings()
```

</details>

### 4.1 Skeleton (~30 min)

- `eframe = "0.36"` is already in Cargo.toml. First compile of it takes minutes; that's once.
- In `settings.rs`: `struct SettingsApp { config_file: ConfigFile, draft: Config, error: Option<String> }`
  — `draft` is the working copy, loaded once via `config_file.read()?`.
- `run` body: `eframe::run_native(...)` with a small fixed-ish window (`ViewportBuilder` size
  ~480×360), creator closure builds `SettingsApp`. Copy the wiring shape from the docs.rs/eframe
  front page, then type your own.
- Map eframe's error into `AppError` (one `map_err`; its error type doesn't warrant a variant).
- Checkpoint: tray → Open Settings → empty egui window appears; close it; process gone.

<details>
<summary>Spoiler: 4.1 reference implementation (settings.rs)</summary>

```rust
use eframe::egui;

use crate::{
    config::{Config, ConfigFile},
    error::AppError,
};

struct SettingsApp {
    config_file: ConfigFile,
    draft: Config,
    error: Option<String>,
}

impl eframe::App for SettingsApp {
    // eframe 0.36: the trait method is `ui` (older docs/examples show `update`),
    // and panels are shown inside the provided `Ui`, not from a Context.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("PastaHandler — Snippets");
        });
    }
}

pub fn run(config_file: ConfigFile) -> Result<(), AppError> {
    let draft = config_file.read()?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([520.0, 380.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PastaHandler Settings",
        options,
        Box::new(move |_cc| Ok(Box::new(SettingsApp { config_file, draft, error: None }))),
    )
    .map_err(|e| AppError::Config(format!("settings window failed: {e}")))
}
```

`egui` comes re-exported through eframe (`use eframe::egui;`) — no separate dependency line.

</details>

### 4.2 The table (~1 evening)

- `egui::Grid` (header row: Label / Text / Hotkey / actions) — each snippet row: two `TextEdit`s,
  one hotkey `TextEdit`, a Delete button. Below: an Add button pushing a default `Snippet`.
- Immediate-mode recap: you re-declare the whole UI every frame in `update()`; widgets return
  interaction state; all state lives in `SettingsApp`. Nothing is "bound" — you read/write
  `self.draft.snippets[i].label` directly in the loop.
- Deleting while iterating: collect the index into an `Option<usize>` during the loop, remove
  *after* it — you can't mutate the Vec you're iterating (the borrow checker will explain).

<details>
<summary>Spoiler: 4.2 reference implementation (update() body)</summary>

```rust
fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("PastaHandler — Snippets");
        ui.add_space(8.0);

        let mut delete: Option<usize> = None;

        egui::Grid::new("snippets")
            .striped(true)
            .num_columns(4)
            .show(ui, |ui| {
                ui.strong("Label");
                ui.strong("Text");
                ui.strong("Hotkey");
                ui.strong("");
                ui.end_row();

                for (i, snippet) in self.draft.snippets.iter_mut().enumerate() {
                    ui.text_edit_singleline(&mut snippet.label);
                    ui.text_edit_singleline(&mut snippet.text);
                    ui.text_edit_singleline(&mut snippet.hotkey);
                    if ui.button("Delete").clicked() {
                        delete = Some(i);
                    }
                    ui.end_row();
                }
            });

        if let Some(i) = delete {
            self.draft.snippets.remove(i);   // after the loop — the borrow is released here
        }

        ui.add_space(8.0);
        if ui.button("Add snippet").clicked() {
            self.draft.snippets.push(crate::config::Snippet {
                label: "New".into(),
                text: String::new(),
                hotkey: String::new(),
            });
        }
    });
}
```

Checkpoint for this step: rows render, edits stick between frames (state lives in `self.draft`),
Add/Delete work. Nothing writes to disk yet — that's 4.3.

</details>

### 4.3 Commit-and-write (the auto-save) (~1 evening)

- **Commit point** = a field losing focus (`response.lost_focus()`) or Enter
  (`ui.input(|i| i.key_pressed(egui::Key::Enter))`), or Add/Delete clicked. NOT every keystroke —
  you'd write half-typed hotkeys.
- On commit: **validate, then write.** Validation = parse every hotkey
  (`snippet.hotkey.parse::<HotKey>()` — same parser the resident uses) + `HashSet` duplicate check.
  Valid → `self.config_file.write(&self.draft)` (already atomic: temp + rename). Invalid → set
  `self.error = Some(msg)`, show it in red under the table (`ui.colored_label`), and *don't write* —
  the file keeps its last good state, so the resident never even sees the bad edit.
- This mirrors the resident's own validate-first reload: both processes refuse to act on a config
  they haven't validated. Belt at both ends, file always sane in between.
- Checkpoint (this is the v1 payoff moment): resident running → Open Settings → add a snippet with
  a fresh hotkey → click away (commit) → press the hotkey → **it pastes, no restart, no Notepad,
  no hand-edited TOML.** Type a garbage hotkey → red error, file untouched, resident unaffected.

<details>
<summary>Spoiler: 4.3 reference implementation (validation shared with the resident + commit-and-write)</summary>

First, a tiny extraction so both processes validate with the *same* code — in `hotkeys.rs`, pull
pass 1 of `register_all` out into a free function:

```rust
// hotkeys.rs
/// Pass 1 of registration, shared with the settings UI: parse + duplicate-check
/// every hotkey. No side effects — safe to call on any draft.
pub fn parse_all(config: &Config) -> Result<Vec<(HotKey, usize)>, AppError> {
    let mut parsed = Vec::new();
    let mut seen = HashSet::new();
    for (i, snippet) in config.snippets.iter().enumerate() {
        let hotkey: HotKey = snippet.hotkey.parse().map_err(|_| {
            AppError::Config(format!("bad hotkey '{}' on '{}'", snippet.hotkey, snippet.label))
        })?;
        if !seen.insert(hotkey.id()) {
            return Err(AppError::Config(format!("duplicate hotkey '{}'", snippet.hotkey)));
        }
        parsed.push((hotkey, i));
    }
    Ok(parsed)
}
```

…and `register_all`'s pass 1 becomes one line: `let parsed = parse_all(config)?;`

Then in `update()`, capture commit signals and act after the UI is declared:

```rust
// inside the Grid row loop — the three text edits now report commits:
let r1 = ui.text_edit_singleline(&mut snippet.label);
let r2 = ui.text_edit_singleline(&mut snippet.text);
let r3 = ui.text_edit_singleline(&mut snippet.hotkey);
committed |= r1.lost_focus() || r2.lost_focus() || r3.lost_focus();
// (declare `let mut committed = false;` next to `delete`, before the Grid.
//  lost_focus() fires on Enter too — no separate key check needed.)

// Delete also commits:
if let Some(i) = delete {
    self.draft.snippets.remove(i);
    committed = true;
}
// Add deliberately does NOT commit — a fresh row has an empty hotkey; it gets
// written when you fill it in and click away.

// after the Add button, the commit handler:
if committed {
    match crate::hotkeys::parse_all(&self.draft) {
        Ok(_) => {
            self.error = None;
            if let Err(e) = self.config_file.write(&self.draft) {
                self.error = Some(e.to_string());
            }
        }
        Err(e) => self.error = Some(e.to_string()),   // file untouched — last good state stands
    }
}

if let Some(err) = &self.error {
    ui.add_space(8.0);
    ui.colored_label(egui::Color32::RED, err);
}
```

</details>

### 4.4 Scope contract (unchanged, non-negotiable)

One window, one screen: table + Add + inline error. No theming, no tray notifications, no
import/export, no hotkey-recorder widget (a validated text field IS v1), no conflict wizard,
no single-instance guard. All parked in v2.

---

## Segment 5 — Packaging · 1 evening

1. `[profile.release] strip = true`; `.cargo/config.toml` with
   `rustflags = ["-C", "target-feature=+crt-static"]` (no VC++ redist dependency on clean machines).
   `cargo build --release` → sanity-run the exe standalone from another folder.

<details>
<summary>Spoiler: the two config blocks verbatim</summary>

```toml
# Cargo.toml — add:
[profile.release]
strip = true

# .cargo/config.toml (new file, project root):
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

</details>
2. Exe icon (optional polish): generate sizes from `assets/icon.svg` with resvg (installed) —
   16/32/48/256 → pack as `.ico` → `winresource` crate or an `.rc` step. Skippable for v1.
3. Inno Setup (https://jrsoftware.org/isdl.php) → `installer/pastahandler.iss`:

```iss
[Setup]
AppName=PastaHandler
AppVersion=0.1.0
DefaultDirName={autopf}\PastaHandler
OutputBaseFilename=pastahandler-setup-0.1.0
Compression=lzma2
SolidCompression=yes

[Files]
Source: "..\target\release\pastahandler.exe"; DestDir: "{app}"

[Tasks]
Name: "startup"; Description: "Start PastaHandler when Windows starts"; Flags: unchecked

[Icons]
Name: "{autoprograms}\PastaHandler"; Filename: "{app}\pastahandler.exe"
Name: "{userstartup}\PastaHandler"; Filename: "{app}\pastahandler.exe"; Tasks: startup

[Run]
Filename: "{app}\pastahandler.exe"; Description: "Launch now"; Flags: nowait postinstall skipifsilent
```

4. User-facing `README.md` — must contain:
   - **SmartScreen walkthrough with screenshots.** Locally built files carry no Mark-of-the-Web —
     to reproduce what users see, upload the installer somewhere and download it back, then run:
     "Windows protected your PC" → *More info* → *Run anyway*. Code-signing kills this in v2
     (OV cert ≈ $100–400/yr; EV / Azure Trusted Signing = immediate trust).
   - **The COMPLIANCE.md §4 usage note:** paste in-context and sparingly — spamming the same link
     every game risks chat restrictions under Riot's spam clause regardless of tooling.
5. Checkpoint: on a machine/VM that's never seen the project — download installer → SmartScreen
   click-through → install → tray icon → add snippet via settings UI → hotkey pastes → uninstall
   cleanly via Settings → Apps.

---

## Verification (before calling v1 done)

- **Unit** (`cargo test`): config round-trip via temp path; hotkey parse rejects garbage as
  `AppError::Config`; clipboard write→read-back marked `#[ignore]`.
- **E2E:** Segment 4 + 5 checkpoints, then League itself — hotkey → `Ctrl+V` in post-game lobby chat.
- **Performance receipts:** Task Manager GPU column: resident = 0% GPU always (game running too);
  settings process appears on open, *gone* after close; idle CPU ≈ 0%.
- **Compliance receipt** (must return nothing):

```bash
grep -rnE "SendInput|keybd_event|SetWindowsHookEx|OpenProcess|ReadProcessMemory|WriteProcessMemory" src/
```

- **Known cosmetic quirk:** a killed (not quit) resident may ghost its tray icon until mouseover —
  Windows behavior, not a bug.

## v2 parking lot (write ideas here, don't build them)

Code signing · exe `.ico` if skipped · hotkey-recorder widget · single-instance guards (resident
named-mutex; settings window focus-instead-of-second) · `notify` crate over mtime polling ·
import/export · log file next to config.toml (reload failures are invisible once the console is
gone — first real v2 item) · `thiserror` migration · cross-platform · **never**: auto-paste.
