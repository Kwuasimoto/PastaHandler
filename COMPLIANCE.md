# TwitchNamePaste (tnp) — Riot TOS Design-Compliance Memo

**Date of clause verification:** 2026-08-16 (all quoted language fetched directly from riotgames.com on this date)

---

## 1. What this document is — and is not

This is a **good-faith design-compliance memo**: it maps every architectural decision in tnp against the
specific clauses of Riot Games' Terms of Service and Community Pact that govern third-party software.

It is **not legal advice** (no lawyer wrote it), and it is **not a shield**. Riot's ToS §2.1.2 gives Riot
discretion to suspend accounts where they "reasonably determine" a violation, and platform enforcement is
not a court. What this memo *is* good for:

1. **Design discipline** — every capability tnp has (and deliberately lacks) traces to a clause below.
2. **Evidence of intent** — if an account is ever falsely flagged, this memo plus the greppable design
   receipts (§6) are ready-made material for a Riot Support appeal.

## 2. What tnp is (the facts this memo relies on)

tnp is a Windows system-tray utility. The user assigns global hotkeys to short text snippets. When a
hotkey is pressed, tnp writes that snippet to the **Windows clipboard**. Nothing else happens. The user
then pastes it themselves with a real, physical `Ctrl+V` keypress — in League chat or anywhere else.

Everything tnp does happens at the OS level, outside any game:

| tnp does | tnp never does |
|---|---|
| Registers hotkey combos with Windows via `RegisterHotKey` | Installs keyboard hooks (`SetWindowsHookEx`) |
| Receives `WM_HOTKEY` for its own registered combos only | Observes any other keystroke |
| Writes text to the OS clipboard (`SetClipboardData` via `arboard`) | Sends synthetic input (`SendInput` / `keybd_event`) |
| Shows a tray icon and a settings window | Opens a handle to any Riot process (`OpenProcess`) |
| Reads/writes its own config file in `%APPDATA%\tnp\` | Reads or writes any process's memory (`ReadProcessMemory` / `WriteProcessMemory`) |
| | Touches Riot files, windows, network traffic, or APIs |
| | Loads any kernel driver |

The paste itself is always the human's own keypress. tnp has **zero visibility into, and zero interaction
with, the game.**

## 3. The governing clauses (verbatim, verified 2026-08-16)

### Riot Terms of Service — Section 7 "USER RULES"

**§7.1(11)** — the anti-cheat / automation clause:

> "Using any unauthorized third party programs, including mods, hacks, cheats, scripts, bots, trainers
> and automation programs **that interact with the Riot Services in any way**, for any purpose, including
> any unauthorized third party programs that intercept, emulate, or redirect any communication relating
> to the Riot Services and any unauthorized third party programs that collect info about the Riot
> Services by reading areas of memory used by the Riot Services to store info"

**§7.1(9)** — the spam clause:

> "Spamming chat, whether for personal or commercial purposes, by disrupting the flow of conversation
> with repeated postings"

**§2.1.2** — enforcement discretion:

> "We may terminate or suspend your account if we reasonably determine, that: [...]"

…and Riot "reserve[s] the right to terminate any other accounts you may have created."

Source: https://www.riotgames.com/en/terms-of-service
*(Note: item numbering within §7.1 can shift across ToS revisions and regions; the quoted language is
what governs. Re-verify wording against the live page if this memo is ever actually needed.)*

### Riot Community Pact

**"Play Fair"** prohibits:

> "Cheating—including scripting and botting"
> "Using mods, hacks, cheats, or other programs that give you an unfair advantage"

**Glossary — Riot's own definition of Scripting:**

> "Scripting: When a player uses 3rd party software (or hardware) to take automated actions (ie:
> auto-aiming or auto-dodging) or respond to in-game events on their behalf."

**"Play with Respect"** prohibits:

> "Spamming comms channels with excessive noise or off-topic behavior"

**Penalty ladder** (verbatim): "Warnings – Cautionary messages with no sanctions. Delays – Delay of play
for a period of time. Restrictions – Temporary limitation or loss of access to a specific feature, like
in-game chat or other communication features. Time-based Suspensions – Temporary prevention of play.
Permanent Account Suspensions – Permanent prevention of play (single account). Hardware Bans - Permanent
prevention of play (physical device)." — "Penalties scale with the severity of the offense and your
history of behavior."

Source: https://www.riotgames.com/en/community-pact

## 4. Clause-by-clause analysis

### §7.1(11): the clause's own qualifier excludes tnp

The prohibition is scoped by its own text: automation programs "**that interact with the Riot Services in
any way**." tnp interacts with the Windows clipboard and the Windows hotkey registry. It does not
interact with the Riot Services in any way. Taking each sub-prong of §7.1(11) in turn:

- *"intercept, emulate, or redirect any communication relating to the Riot Services"* — tnp opens no
  network sockets, reads no traffic, and emulates nothing. It has no networking code at all.
- *"collect info about the Riot Services by reading areas of memory used by the Riot Services"* — tnp
  never opens a handle to any process. There is no `OpenProcess`, no `ReadProcessMemory`, anywhere.
- *"mods, hacks, cheats, scripts, bots, trainers"* — tnp modifies no game files, hooks no game code,
  automates no gameplay, and grants no gameplay capability.

The remaining word is "automation programs." tnp automates exactly one thing: **filling the user's own
clipboard** — an OS feature, outside the game. The paste into chat remains a deliberate human keypress.
Whether that final word could be stretched over tnp is the grey zone; the qualifier "that interact with
the Riot Services in any way" is the textual reason it should not be.

### Community Pact "Scripting": excluded by Riot's own definition

Riot defines Scripting as software that takes "automated actions (ie: auto-aiming or auto-dodging) **or
respond[s] to in-game events** on their behalf." tnp:

- takes no in-game actions, automated or otherwise;
- *cannot* respond to in-game events — it has no visibility into the game whatsoever;
- responds only to the user's deliberate, physical hotkey press, and its only effect is on the OS
  clipboard.

### "Unfair advantage": none conferred

Pre-loading one's own text into one's own clipboard confers no gameplay advantage. For calibration:
Riot *approves* companion apps that actively read game data and render overlays (Blitz.gg, Porofessor).
tnp is strictly less invasive than software Riot has explicitly blessed — it reads nothing at all.

### §7.1(9): the honest residual risk — and it attaches to usage, not to the software

"Spamming chat … with repeated postings" is the one clause tnp's *use* can genuinely violate. Pasting
the same Twitch link into chat game after game is exactly "repeated postings … for personal or
commercial purposes." Key points:

- This is a **player-behavior** matter, enforced through the chat-moderation ladder (restrictions first,
  escalating with repetition and history) — entirely separate from Vanguard/anti-cheat.
- No Riot rule prohibits posting a stream link per se; the trigger is repetition/disruption.
- **The tool cannot make this judgment for the user.** The README ships with an explicit note: use it
  in-context and sparingly; spamming the link risks chat restrictions regardless of how the text got
  into the clipboard.

## 5. Anti-cheat (Vanguard) posture

Riot does not publish Vanguard internals, but the documented enforcement picture (Riot's own /dev
retrospective plus community analysis) ranks operations roughly least → most likely to draw action:

1. Writing the OS clipboard — *no documented enforcement*
2. `RegisterHotKey` — *no documented enforcement; no hook, no injection, sees only its own combo*
3. Low-level keyboard hook (`WH_KEYBOARD_LL`) — *no documented enforcement, but keylogger-shaped*
4. **— the cliff —**
5. Synthetic input into the game (`SendInput` etc.) — *documented enforcement target (macros/autoclickers)*
6. Reading/writing game memory — *the core thing Vanguard exists to catch*

tnp lives entirely at ranks 1–2 **by design**, including deliberately choosing `RegisterHotKey` over the
broader hook API even though the hook would have been easier to code against. Riot's Vanguard x LoL
retrospective states false-positive bans run below 0.01% (fewer than 1 in 10,000 bans) with innocent
accounts restored in under 72 hours on average, and that benign apps which trip Vanguard's protections
get *blocked from functioning*, not banned. tnp performs none of the operations (handle-opening,
hooking into the client) that even the blocked category exhibits.

Source: https://www.leagueoflegends.com/en-us/news/dev/dev-vanguard-x-lol-retrospective/

## 6. Design receipts (verifiable in the source)

The claims above are checkable against the codebase mechanically. This must return **no matches**:

```bash
grep -rnE "SendInput|keybd_event|SetWindowsHookEx|OpenProcess|ReadProcessMemory|WriteProcessMemory" src/
```

Dependency receipts (`Cargo.toml` / `Cargo.lock`):

- `global-hotkey` — wraps Win32 `RegisterHotKey`/`UnregisterHotKey` (narrow: the OS delivers only the
  registered combos; the process never observes other keystrokes).
- `arboard` — wraps the Win32 clipboard API (`clipboard-win` backend).
- No input-simulation crate (`enigo`, `inputbot`, `rdev`, …) is present, and none may ever be added to
  v1 — auto-paste is explicitly out of scope *because* it would cross the §7.1(11) line.

## 7. Limits and residual risk — stated plainly

1. **Discretion:** §2.1.2 lets Riot act on what they "reasonably determine." No memo binds them.
2. **Policy drift:** Riot publishes principles, not an allow-list, and has widened enforcement before —
   in March 2025 they banned even *read-only* enemy-ult-timer overlays. A future policy could sweep
   more broadly. Re-check the live ToS periodically.
3. **False flags:** rare and short-lived per Riot's own numbers, but nonzero. That is what §8 is for.
4. **The spam clause is real:** the most likely bad outcome for a tnp user is a chat restriction earned
   through overuse — self-inflicted, and outside the software's control.

## 8. If an account is ever flagged (appeal playbook)

1. File a ticket at https://support.riotgames.com — category: account suspension appeal.
2. State what tnp is in one sentence: *a clipboard utility that writes user-authored text to the Windows
   clipboard on a hotkey; it never interacts with the game process, sends no input, and reads no memory.*
3. Attach or link: this memo, the source repository, and the §6 grep receipt.
4. Note the design choices made specifically to stay compliant: `RegisterHotKey` (not hooks), no
   synthetic input, manual paste only.

---

*Clause language verified 2026-08-16 against riotgames.com/en/terms-of-service and
riotgames.com/en/community-pact. If you are reading this long after that date, re-verify before relying
on it.*
