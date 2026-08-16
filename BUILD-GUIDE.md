# TwitchNamePaste (tnp) — Hand-Coding Build Guide

**You write every line.** This guide gives you module layout, type signatures, API names, and short
snippets *only* where the shape is genuinely unguessable after a year away. It never hands you a full
implementation. Each segment ends in a checkpoint — **do not move on until the checkpoint passes**, and
tackle exactly one segment at a time.

**Companion doc:** [COMPLIANCE.md](COMPLIANCE.md). Its design rules are load-bearing: no
`SetWindowsHookEx`, no `SendInput`/`keybd_event`, no `OpenProcess`/`ReadProcessMemory`, no
input-simulation crates, no auto-paste. Ever.

## Performance posture (why the app is shaped the way it is)

tnp is two processes sharing one `.exe`:

- **Resident process** (`tnp.exe`) — hotkeys + tray + clipboard on a bare `tao` event loop. **No
  window, no GPU context, ever.** Idles at ~1 wake/second, single-digit MB of RAM. This is the only
  thing running while a game runs, and it is deliberately indistinguishable from nothing.
- **Settings process** (`tnp.exe --settings`) — a plain `eframe`/egui window in its own short-lived
  process. Opens when you ask, edits `config.toml`, exits fully on close — its GPU context dies with
  it. The resident process notices the config file changed and re-registers hotkeys.

Why not one process with a hidden settings window: a hidden eframe window keeps a Direct3D/wgpu
context, swapchain, and font atlas resident in VRAM mid-game for zero benefit. Two processes cost
nothing, keep the resident half GPU-free by construction, and — bonus — completely eliminate the
"two event loops, one thread" integration problem. Both halves stay boring, simple code.

## The stack (versions verified on crates.io, Aug 2026)

```toml
[dependencies]
global-hotkey = "0.8"   # Win32 RegisterHotKey — narrow; OS delivers only YOUR combos
tray-icon     = "0.24"  # tray + menu (muda); sibling crate of global-hotkey
arboard       = "3.6"   # clipboard; on Windows, contents outlive your process
tao           = "0.36"  # the resident process's Win32 event loop
eframe        = "0.36"  # settings process ONLY
serde         = { version = "1", features = ["derive"] }
toml          = "1.1"
```

Why not Tauri: WebView2 + a JS frontend for a tray tool is a cathedral where the spec says shed. We
use Tauri's *building blocks* (`global-hotkey`, `tray-icon`) directly. The `image` crate arrives in
Segment 3 for the tray icon.

**Ground rule:** when a snippet here doesn't compile, the crates' own `examples/` folders on GitHub
and docs.rs front pages are the source of truth — these APIs move, this file doesn't.

## Segment map

| Segment | What you ship | Honest time |
|---|---|---|
| 0 | Environment + hello-world GUI installed via a real installer | 1 focused day (fine split over 2–3 evenings) |
| 1 | Resident skeleton: hardcoded hotkey → clipboard | 1 evening |
| 2 | Config file + real error handling | 1–2 evenings |
| 3 | Tray icon; app becomes invisible | 1 evening |
| 4 | Settings window (separate process) | 2–3 evenings |
| 5 | Final packaging + README | 1 evening |

---

## Segment 0 — Environment → hello-world GUI → installable .exe

**Goal:** prove the ENTIRE pipeline — toolchain → GUI window → release build → installer →
install/uninstall — while the code is still trivial. Every scary unknown dies here, not in Segment 5.

### 0.1 Install rustup (~5 min)

1. Go to https://rustup.rs → download `rustup-init.exe` → run it.
2. It will show the default: `stable-x86_64-pc-windows-msvc`. That `msvc` suffix is what you want
   (the GNU alternative exists; ignore it — MSVC is the native Windows target). Press Enter for the
   default install.
3. What you just installed, in one line each:
   - `rustup` — toolchain manager (updates Rust, switches versions)
   - `cargo` — build tool + package manager; the only command you'll actually type all day
   - `rustc` — the compiler; cargo drives it, you almost never call it directly
4. **Verify** in a NEW terminal (PATH changes need a fresh shell):

```bash
cargo --version
```

   Expect something like `cargo 1.8x.0 (...)`. If "not recognized": open a new terminal first, then
   check `%USERPROFILE%\.cargo\bin` is on PATH.
5. Already had rustup from years ago? Just run:

```bash
rustup update
```

### 0.2 The MSVC linker — the classic time sink (15–45 min, 2–8 GB download)

Rust compiles your code, but *linking* the final .exe uses Microsoft's `link.exe`, which ships with
Visual Studio's C++ tools. This is the step that historically eats a day when skipped.

1. During rustup install, recent versions **detect the missing tools and offer to install them for
   you** — if you got that prompt and accepted, you're likely done; skip to the verify step.
2. Manual route: https://visualstudio.microsoft.com/downloads/ → scroll to **"Build Tools for Visual
   Studio 2022"** (free; it's the compiler tools *without* the Visual Studio IDE) → run installer →
   check exactly one workload: **"Desktop development with C++"** (this includes the Windows SDK) →
   Install. Big download; go make coffee.
3. **The canonical failure**, so you recognize it instead of panicking:

   ```text
   error: linker `link.exe` not found
   ```

   This always means step 2 is missing or incomplete. Re-run the Build Tools installer → Modify →
   confirm "Desktop development with C++" is checked.
4. **Verify** — don't trust, build (this is also your 0.4 warm-up):

```bash
cargo new linkcheck && cd linkcheck && cargo run
```

   Expect: a compile line, then `Hello, world!`. If that printed, your toolchain is DONE — the
   hardest environment step is behind you. Delete the `linkcheck` folder.

### 0.3 Editor (~10 min)

- VS Code → Extensions → install **rust-analyzer** (the official language server: inline types,
  errors as you type, go-to-definition). That's the whole setup.
- Optional but worth it: **CodeLLDB** extension for breakpoint debugging later.
- Tip while relearning: rust-analyzer's inlay type hints are training wheels that actually teach —
  leave them on.

### 0.4 Console hello world — crate anatomy (~10 min)

You built one in 0.2; now actually look at it. You're already *in* the project directory
(`twitchnamepaste`, alongside the two docs), so initialize the crate **in place** — don't `cargo new`
(that would nest a folder):

```bash
cargo init --name tnp
```

- `init` sets up the current directory as the crate; existing files (COMPLIANCE.md, BUILD-GUIDE.md)
  are untouched.
- `--name tnp` overrides the folder-derived package name so the exe comes out as `tnp.exe`, not
  `twitchnamepaste.exe`.
- It also `git init`s the folder if it isn't a repo (it isn't). Feature, not accident — commit at
  every checkpoint.

Anatomy — one line each, this is the whole mental model:

- `Cargo.toml` — manifest: name, version, dependencies. You edit this.
- `src/main.rs` — entry point: `fn main()`.
- `.gitignore` — cargo generated it, already ignores `target/`.
- `target/` — ALL build output; gigabytes eventually; never commit, delete freely (`cargo clean`).
- `Cargo.lock` — exact dependency versions; commit it for a binary project.

Your feedback loops, fast to slow: `cargo check` (type-checks only — your main loop while writing),
`cargo run` (debug build + run), `cargo build --release` (slow, optimized — only for shipping).

### 0.5 Hello-world GUI (~45–60 min — mostly one big first compile)

This window is **not throwaway** — in Segment 4 it grows into the settings UI.

1. In `Cargo.toml`, add under `[dependencies]`:

```toml
eframe = "0.36"
```

   (Verified current Aug 2026; if you're reading this later, use whatever docs.rs/eframe says is
   latest stable.)
2. **Expectation-setting:** the first `cargo run` after adding eframe downloads and compiles a few
   hundred crates and takes several *minutes*. This is once. After that, incremental rebuilds are
   seconds. Do not conclude something is broken at minute three.
3. Now hand-write the minimal app. The shape (the up-to-date version of this exact skeleton is the
   first example on docs.rs/eframe — cross-check there, then type your own):

```rust
struct HelloApp { clicks: u32 }

impl eframe::App for HelloApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello, tnp");
            if ui.button("Click me").clicked() { self.clicks += 1; }
            ui.label(format!("Clicks: {}", self.clicks));
        });
    }
}
```

   …launched from `main` via `eframe::run_native(...)` with default `NativeOptions` and a closure
   that constructs your app. Two things to absorb while typing:
   - **Immediate mode:** `update()` re-declares the entire UI every frame; widgets return whether
     they were interacted with. State lives in your struct, never in widgets.
   - The `clicks` counter is there to prove state persists across frames — egui isn't rebuilding
     your struct, just re-drawing from it.
4. `cargo run` → window appears, button counts.

### 0.6 A real release .exe (~15 min)

1. Add to `Cargo.toml`:

```toml
[profile.release]
strip = true
```

2. Kill the console window that flashes behind GUI apps — first line of `main.rs`:

```rust
#![windows_subsystem = "windows"]
```

   Cost: `println!` output now goes nowhere. While developing, comment it out when you need prints.
3. **CRT landmine (defused now, not in Segment 5):** by default MSVC builds dynamically link
   Microsoft's C runtime — your exe can fail on a truly clean PC that lacks the VC++ redistributable.
   Static-link it instead: create `.cargo/config.toml` in the project root:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

4. Build and prove standalone-ness:

```bash
cargo build --release
```

   → `target/release/tnp.exe`. Double-click it in Explorer. Copy it alone to `C:\temp\` and run it
   from there — it must work with nothing beside it.

### 0.7 Mini-installer with Inno Setup (~1–2 h)

1. Install Inno Setup: https://jrsoftware.org/isdl.php (the compiler + a GUI script editor).
2. Create `installer/hello.iss` — this is a complete, working script; type it, compile it in the
   Inno editor (Build → Compile), and read each directive as you go (Inno's F1 help is excellent):

```iss
[Setup]
AppName=TnpHello
AppVersion=0.0.1
DefaultDirName={autopf}\TnpHello
OutputBaseFilename=tnp-hello-setup
Compression=lzma2
SolidCompression=yes

[Files]
Source: "..\target\release\tnp.exe"; DestDir: "{app}"

[Icons]
Name: "{autoprograms}\TnpHello"; Filename: "{app}\tnp.exe"

[Run]
Filename: "{app}\tnp.exe"; Description: "Launch now"; Flags: nowait postinstall skipifsilent
```

3. Run the produced `tnp-hello-setup.exe`: installs to Program Files, Start-menu entry appears,
   app launches from it. Then Settings → Apps → uninstall — confirm it removes cleanly.
4. **SmartScreen honesty:** your locally built installer will NOT trigger SmartScreen — the warning
   is driven by Mark-of-the-Web, which only downloaded files carry. To see what real users see,
   upload the installer somewhere (GitHub release, Drive) and download it back, then run. Expect
   "Windows protected your PC" → *More info* → *Run anyway*. That's the unsigned-v1 experience;
   code-signing is the v2 fix.

### ✅ Segment 0 checkpoint

Hello GUI app, built in release mode, installed through a real installer, launched from the Start
menu, uninstalled cleanly. **Your environment is proven end-to-end. Everything after this is just
Rust.**

---

## Segment 1 — Resident skeleton: hotkey → clipboard

**Goal:** one hardcoded snippet, one hardcoded hotkey, clipboard write. The resident process's whole
spine — event loop → hotkey event → clipboard — in the fewest possible lines.

### Steps

1. Add `global-hotkey`, `arboard`, `tao` to dependencies.
2. Comment out `#![windows_subsystem = "windows"]` for now — you want `println!` back while
   developing. (It returns in Segment 3.)
3. Set the eframe hello code aside — move it into a `settings.rs` module (unused for now; it comes
   back in Segment 4). Add the two-line dispatch at the top of `main`: if
   `std::env::args().any(|a| a == "--settings")`, run the (stubbed) settings app; otherwise run the
   resident loop you're about to write.
4. Write the resident loop in `main.rs`.

### The unguessable wiring

`GlobalHotKeyManager` must live on a thread with a running Win32 event loop, and hotkey events arrive
on a **global channel**, not through the loop. The shape (cross-check `global-hotkey`'s
`examples/tao.rs`):

```rust
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, hotkey::{Code, HotKey, Modifiers}};
use tao::event_loop::{ControlFlow, EventLoopBuilder};

fn main() {
    let event_loop = EventLoopBuilder::new().build();

    let manager = GlobalHotKeyManager::new().unwrap();      // must stay alive — gotcha 1
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Digit1);
    manager.register(hotkey).unwrap();

    let hotkey_rx = GlobalHotKeyEvent::receiver();          // global crossbeam channel

    event_loop.run(move |_event, _target, control_flow| {
        *control_flow = ControlFlow::Poll;                  // heartbeat comes in Segment 3
        if let Ok(event) = hotkey_rx.try_recv() {
            // match event.id() against hotkey.id(), check event.state — then clipboard write
        }
    });
}
```

Clipboard write is two lines with `arboard`: `Clipboard::new()`, `.set_text(...)`. Both return
`Result` — `unwrap()` is fine for this segment; Segment 2 replaces every unwrap with real plumbing.

### Gotchas

1. **Keep the `GlobalHotKeyManager` alive.** If it drops, hotkeys silently unregister. The `move`
   closure capturing it suffices — but refactor it into a function that returns, and it dies. A
   genuinely instructive ownership bug; hit it once on purpose.
2. **Hotkey events fire on press AND release** — check `event.state` for `HotKeyState::Pressed`
   only, or every press writes the clipboard twice.
3. **Clipboard contention:** the Windows clipboard is shared; a write can transiently fail if
   another app holds it open. A 3-attempt/50 ms retry makes it robust — fine to defer to Segment 2
   where error handling lives.
4. `ControlFlow::Poll` spins a core's worth of wakeups — acceptable for this segment only; the
   proper heartbeat arrives with the tray.

### ✅ Checkpoint

`cargo run`, press `Ctrl+Alt+1`, `Ctrl+V` into Notepad → your string appears. Quit the app,
`Ctrl+V` again → **still pastes** (Windows owns clipboard data after `SetClipboardData` — this is
why tnp needs no daemon).

**Re-learned:** ownership/lifetime of a manager object, `move` closures, channels (`try_recv`),
first `Result`s.

---

## Segment 2 — Config: multiple snippets, real error handling

**Goal:** snippets in `%APPDATA%\tnp\config.toml`; all hotkeys registered at startup; every fallible
operation returns a typed `Result`. The module layout and error convention you set here carry to the
end.

### Module layout

```
src/
  main.rs        // dispatch (--settings vs resident); resident loop lives here
  error.rs       // AppError + Result alias
  config.rs      // Snippet, Config, load/save
  hotkeys.rs     // registration map: hotkey id -> snippet index
  clipboard.rs   // write_text(&str) -> Result<()>  (retry lives here)
  settings.rs    // parked eframe app (Segment 4)
```

### The error convention (your global standard, Rust edition)

Rust's `Result` **is** the discriminated union you use in TypeScript — the language enforces the
discrimination. Hand-roll the enum (you're relearning; `thiserror` is the production shortcut to
adopt later, once writing `Display` by hand gets old):

```rust
// error.rs
#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    TomlParse(toml::de::Error),
    TomlWrite(toml::ser::Error),
    Hotkey(global_hotkey::Error),
    Clipboard(arboard::Error),
    Config(String),              // domain errors: bad hotkey string, duplicate combo…
}

pub type Result<T> = std::result::Result<T, AppError>;
```

Write `From<std::io::Error> for AppError` (and one per wrapped type) — that's what makes `?` work,
and `?` is the entire ergonomic payoff. Impl `Display` so errors read like sentences. Your global
rules apply verbatim: propagate what you can't handle, never log-and-ignore, `panic!` only for
invariant violations.

### Config shapes

```rust
// config.rs
#[derive(serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Snippet {
    pub label: String,      // "My Twitch"
    pub text: String,       // "twitch.tv/yourname"
    pub hotkey: String,     // "ctrl+alt+Digit1" — global-hotkey's FromStr format
}

#[derive(serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub struct Config { pub snippets: Vec<Snippet> }

pub fn config_path() -> Result<std::path::PathBuf>;         // %APPDATA%\tnp\config.toml
pub fn load_from(path: &Path) -> Result<Config>;            // missing file => Ok(default)
pub fn save_to(path: &Path, config: &Config) -> Result<()>; // create_dir_all first
```

(Path-taking functions + thin `%APPDATA%` wrappers = testable against temp dirs. Oldest trick
there is.)

Facts you'd otherwise hunt for:

- `std::env::var("APPDATA")` → roaming AppData. Returns `Result`; plumb it.
- **`HotKey` implements `FromStr`**: `"ctrl+alt+Digit1".parse::<HotKey>()`. Key names are W3C
  UI-Events codes (`Digit1`, `KeyQ`, `F5`); modifiers `ctrl`/`alt`/`shift`/`super`, case-insensitive.
  String form in TOML keeps the config hand-editable. Map parse failures into `AppError::Config`
  naming the offending string.
- `toml::from_str` / `toml::to_string_pretty` once serde derives are on.

### Registration map

`hotkeys.rs`: `register_all(manager: &GlobalHotKeyManager, config: &Config) ->
Result<HashMap<u32, usize>>` — parse each snippet's combo, register, map `hotkey.id()` → snippet
index. Resident handler becomes: `event.id()` → map → snippet → `clipboard::write_text`. Reject
duplicates with a `HashSet` → `AppError::Config`. Also: `manager.register()` errors if *any* app on
the system already owns that combo — surface that message readably; it will happen.

### ✅ Checkpoint

Hand-write a config.toml with two snippets → both paste. Break the TOML on purpose (bad hotkey
string, syntax error) → readable error naming the problem, not a panic.

**Re-learned:** modules/visibility, serde derive, `From` + `?`, enum error design,
`HashMap`/`HashSet`, domain vs wrapped errors.

---

## Segment 3 — Tray: the app disappears

**Goal:** no console; tnp lives in the tray with **Open Settings** (spawns the settings process —
stub UI for now) and **Quit**.

### Steps

1. Add `tray-icon = "0.24"` and `image = "0.25"`. Make/find a 32×32 PNG at `assets/icon.png`.
2. `tray.rs`: build icon + menu, return the `TrayIcon` handle to `main.rs` — same rule as the hotkey
   manager: **drop the handle and the icon vanishes.**
3. Wire menu events into the loop; **Open Settings** spawns the second process:

```rust
std::process::Command::new(std::env::current_exe()?).arg("--settings").spawn()?;
```

   (Settings still shows the hello window from Segment 0 — that's fine; it proves the spawn.)
4. Replace `ControlFlow::Poll` with the real heartbeat:
   `ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1))`. Hotkey presses and menu clicks
   arrive as thread messages and wake the loop instantly on their own — the 1 s heartbeat exists only
   for Segment 4's config-file check. Idle CPU: ~0%.
5. Re-enable `#![windows_subsystem = "windows"]`.

### The unguessable parts

- Icon: `tray_icon::Icon::from_rgba(rgba, w, h)`; get bytes via
  `image::load_from_memory(include_bytes!("../assets/icon.png"))` → `.to_rgba8()` → `.into_raw()`
  + dimensions (the pattern tray-icon's own examples use).
- Menu (re-exported `muda` API):

```rust
use tray_icon::{TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};

let menu = Menu::new();
let open_item = MenuItem::new("Open Settings", true, None);  // keep for id comparison
let quit_item = MenuItem::new("Quit", true, None);
menu.append_items(&[&open_item, &quit_item]).unwrap();

let _tray = TrayIconBuilder::new()
    .with_menu(Box::new(menu))
    .with_tooltip("TwitchNamePaste")
    .with_icon(icon)
    .build().unwrap();
```

- Menu clicks: another global channel — `MenuEvent::receiver()`, `try_recv()` in the same loop,
  compare `event.id` to `quit_item.id()`; Quit → `*control_flow = ControlFlow::Exit`.

### Gotchas

- Build the tray on the event-loop thread (you are, if it's constructed in `main` before `run` —
  don't get clever with threads).
- With `windows_subsystem` back on, `println!` is gone — comment the attribute out when debugging;
  a logging framework for a tool this size is v2 gold-plating.

### ✅ Checkpoint

No console; icon in tray; hotkeys still paste; tooltip on hover; Open Settings spawns the hello
window as a separate process (watch it appear and vanish in Task Manager); Quit exits cleanly.

**Re-learned:** `include_bytes!`, handle-lifetime bugs, multiplexing channels through one loop,
process spawning.

---

## Segment 4 — Settings window (its own process)

**Goal:** grow the Segment-0 hello window into the real settings UI. Because it's a separate
process, there is **no event-loop integration problem** — this is a bog-standard eframe app that
reads and writes a TOML file.

### Settings side (`settings.rs`)

- App state: a `Config` (loaded at startup via Segment 2's `load_from`), plus edit-buffer fields.
- UI, one screen: snippet table (label / text / hotkey), Add / Edit / Delete, Save. `egui::Grid`
  does the table; `TextEdit` for fields.
- **Hotkey field is a validated text input, not a key recorder.** Format `ctrl+alt+Digit1`,
  validated on Save with the *same* `.parse::<HotKey>()` from Segment 2 — reuse, don't rewrite.
  Inline error string on failure. (A press-keys-now recorder reads raw keyboard state — exactly the
  smell this app exists to avoid, and a rabbit hole. v2, maybe never.)
- Duplicate-combo guard: same `HashSet` logic — reuse it from `hotkeys.rs`.
- **Save atomically:** write to `config.toml.tmp`, then `std::fs::rename` over the real file.
  Rename-on-same-volume is atomic; the resident process can never observe a half-written file.

### Resident side (small addition)

On each 1 s heartbeat: `fs::metadata(&path)?.modified()` → compare to the last-seen mtime → if
changed: reload config, unregister old hotkeys, register new (keep the old `Vec<HotKey>` around;
`GlobalHotKeyManager` has `unregister` and a bulk `unregister_all` — check the current signature),
swap the id→index map. This also picks up hand-edited TOML for free.

### The anti-scope-creep contract

One window, one screen, the widgets named above. No theming, no tray notifications, no
import/export, no conflict-resolution wizard, no single-instance guard. Anything more is v2 —
you're here to ship, not decorate.

### ✅ Checkpoint

Full loop, no hand-edited TOML: tray → Open Settings → add snippet + hotkey → Save → *without
restarting anything* the hotkey works → `Ctrl+V` pastes. Edit text → paste reflects it. Delete →
hotkey dead. Close settings → process gone from Task Manager; resident unaffected. Duplicate hotkey
→ inline error. Quit → resident exits.

**Re-learned:** immediate-mode UI state, file-based IPC between processes, atomic writes, and that
the simplest coordination mechanism (a file + mtime) is usually enough.

---

## Segment 5 — Final packaging

**Goal:** the real installer. Short segment — you learned Inno in 0.7; this is adaptation.

1. Copy `installer/hello.iss` → `installer/tnp.iss`; update names/version; add the optional
   start-with-Windows task:

```iss
[Tasks]
Name: "startup"; Description: "Start TwitchNamePaste when Windows starts"; Flags: unchecked

[Icons]
Name: "{userstartup}\TwitchNamePaste"; Filename: "{app}\tnp.exe"; Tasks: startup
```

2. Write the user-facing `README.md`. It MUST contain:
   - **SmartScreen walkthrough** with screenshots (download your own installer to reproduce it —
     see 0.7): "Windows protected your PC" → *More info* → *Run anyway*. Normal for unsigned
     software; signing is v2 (OV cert ≈ $100–400/yr; EV / Azure Trusted Signing kills the warning
     immediately; reputation accrues per identity, so start signing before wide distribution).
   - **The usage note from COMPLIANCE.md §4:** paste in-context and sparingly — spamming the same
     link every game risks chat restrictions under Riot's spam clause regardless of how the text got
     into the clipboard.

### ✅ Checkpoint

On a machine/VM/second account that has never seen the project: download the installer → SmartScreen
click-through → install → tray icon → add snippet → hotkey → paste → uninstall cleanly.

---

## Verification (before calling v1 done)

**Unit tests** (`cargo test`):
- Config round-trip: build a `Config`, `save_to` a temp path (`std::env::temp_dir()`), `load_from`,
  assert equality (`PartialEq` already derived).
- Hotkey parsing: valid strings parse; garbage → `AppError::Config`, never a panic.
- Clipboard write→read-back — mark `#[ignore]` (clipboard tests misbehave headless/parallel); run
  explicitly via `cargo test -- --ignored`.

**Manual E2E:** Segment 4 + 5 checkpoints, then the real thing — League open, hotkey, `Ctrl+V` in
chat (post-game lobby is a friendlier first test than mid-match).

**Performance receipts** (the posture, verified):
- Task Manager → Details → add the GPU column: resident `tnp.exe` shows **0% GPU / no GPU memory**,
  including while a game runs.
- Idle CPU ≈ 0.0%; RAM single-digit MB.
- Settings process appears on Open Settings and is *gone* from Task Manager after close.

**Compliance receipt** (COMPLIANCE.md §6) — must return nothing:

```bash
grep -rnE "SendInput|keybd_event|SetWindowsHookEx|OpenProcess|ReadProcessMemory|WriteProcessMemory" src/
```

---

## v2 parking lot (write ideas here instead of building them)

Code-signing cert · hotkey-capture widget · single-instance guard for settings · `notify` crate
instead of mtime polling · import/export · logging · `thiserror` migration · cross-platform (all
core crates already are — the door is open) · **never**: auto-paste.
