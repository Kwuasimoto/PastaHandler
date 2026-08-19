//! The snippet list region: empty-state prompt or the scrolling table with
//! per-row toggle, text edit, hotkey capture, and ghost delete. Owns its two
//! pieces of cross-frame state. Future ask "add a column" lands here.

use eframe::egui;

use super::style::Palette;
use super::widgets::toggle_switch;
use crate::config::Snippet;

pub struct SnippetTable {
    /// Row currently listening for a hotkey combo, if any (the capture state machine).
    capturing: Option<usize>,
    /// Row whose delete button was hovered last frame — drives its red tint;
    /// the tint must be chosen before this frame's hover state exists.
    hovered_delete: Option<usize>,
    /// Why the last capture keypress was rejected — replaces the button's
    /// "press keys…" prompt so the feedback appears exactly where the user is
    /// already looking. Cleared when a new capture is armed.
    capture_hint: Option<&'static str>,
}

#[derive(Default)]
#[must_use]
pub struct TableOutput {
    /// Toggle flip, capture success, text-edit blur, or delete — anything persistable.
    pub committed: bool,
    /// Row activated THIS frame — feeds the commit flow's conflict rollback.
    pub just_activated: Option<usize>,
}

impl SnippetTable {
    pub fn new() -> Self {
        Self { capturing: None, hovered_delete: None, capture_hint: None }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        snippets: &mut Vec<Snippet>,
        palette: &Palette,
        mascot: egui::ImageSource<'static>,
        error_shown: bool, // last frame's error, read pre-commit — sizes the scroll reserve
    ) -> TableOutput {
        let mut out = TableOutput::default();

        // fixed columns + flexible Text column that absorbs the remaining width.
        // 6px gutter: the inputs' own inner padding already adds optical air,
        // so a 10px gutter read as a hole between the text and its hotkey
        const SPACING: f32 = 6.0;
        const TOGGLE_W: f32 = 40.0;
        const HOTKEY_W: f32 = 130.0;
        const DELETE_W: f32 = 36.0;
        let text_w = (ui.available_width() - TOGGLE_W - HOTKEY_W - DELETE_W - 3.0 * SPACING)
            .max(210.0);

        if snippets.is_empty() {
            // empty state: a prompt, not a bare table staring into the void
            ui.add_space(28.0);
            ui.vertical_centered(|ui| {
                ui.add(egui::Image::new(mascot).fit_to_exact_size(egui::vec2(63.0, 52.0)));
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("No snippets yet — press +  Add snippet to create one.")
                        .weak(),
                );
            });
            ui.add_space(28.0);
            return out;
        }

        // deferred flags: mutating the Vec mid-iteration is a borrow error;
        // both are applied after the Grid/ScrollArea closures release iter_mut
        let mut delete: Option<usize> = None;
        let mut hovered_delete_now: Option<usize> = None;

        // snippet list scrolls; header above and footer below stay pinned
        let reserve = if error_shown { 66.0 } else { 42.0 };
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height((ui.available_height() - reserve).max(80.0))
            .show(ui, |ui| {
                egui::Grid::new("snippets")
                    .striped(palette.row_stripes)
                    .num_columns(4)
                    // egui's default min_col_width (interact_size.x = 40) silently
                    // widens the 36px delete column, pushing every row 4px past the
                    // right margin. Zero it: the add_sized widths are the sole
                    // sizing authority, so rows fill the width exactly.
                    .min_col_width(0.0)
                    .spacing([SPACING, 8.0])
                    .show(ui, |ui| {
                        ui.label("");
                        ui.strong("Text");
                        // centered over its fixed column, matching the buttons beneath
                        ui.add_sized(
                            [HOTKEY_W, 18.0],
                            egui::Label::new(egui::RichText::new("Hotkey").strong()),
                        );
                        ui.strong("");
                        ui.end_row();

                        for (i, snippet) in snippets.iter_mut().enumerate() {
                            // an unbound snippet can never be active
                            let can_activate = !snippet.hotkey.trim().is_empty();
                            if !can_activate {
                                snippet.active = false;
                            }
                            let was_active = snippet.active;
                            let toggle = toggle_switch(
                                ui,
                                &mut snippet.active,
                                can_activate,
                                palette.accent,
                                palette.knob,
                            );
                            if !can_activate {
                                toggle.on_hover_text("Set a hotkey first");
                            } else if toggle.changed() {
                                out.committed = true;
                                if !was_active && snippet.active {
                                    out.just_activated = Some(i);
                                }
                            }

                            // add_sized forces the cell allocation — plain desired_width
                            // gets clamped by the Grid's cell sizing.
                            let mut text_edit = egui::TextEdit::singleline(&mut snippet.text)
                                .hint_text("Text to paste")
                                .vertical_align(egui::Align::Center);
                            if !snippet.active {
                                text_edit = text_edit.text_color(palette.dim);
                            }
                            let r_text = ui.add_sized([text_w, 26.0], text_edit);
                            // Hotkey cell: a button that captures the next combo when armed.
                            if self.capturing == Some(i) {
                                // a rejected keypress must not fail silently: the
                                // hint takes over the prompt, right under their eyes
                                let (events, held) =
                                    ui.input(|inp| (inp.events.clone(), inp.modifiers));
                                let (prompt, color) = match self.capture_hint {
                                    Some(hint) => (hint, palette.danger),
                                    None if held.ctrl || held.alt || held.shift => {
                                        ("waiting for next key…", palette.accent)
                                    }
                                    None => ("press keys…", palette.accent),
                                };
                                let resp = hotkey_chip(
                                    ui,
                                    HOTKEY_W,
                                    egui::RichText::new(prompt).italics().color(color),
                                );
                                for ev in events {
                                    if let egui::Event::Key {
                                        key,
                                        physical_key,
                                        pressed: true,
                                        repeat: false,
                                        modifiers,
                                        ..
                                    } = ev
                                    {
                                        let key = physical_key.unwrap_or(key); // layout-independent
                                        if key == egui::Key::Escape {
                                            self.capturing = None;
                                            break;
                                        }
                                        // a lone modifier press is a chord in progress,
                                        // not the key of the combo
                                        if is_modifier_key(key) {
                                            self.capture_hint = None; // held-modifier prompt takes over
                                            continue;
                                        }
                                        if !combo_qualifies(&modifiers) {
                                            self.capture_hint = Some("hold Ctrl or Alt");
                                            continue;
                                        }
                                        let combo = combo_from(&modifiers, key);
                                        if combo.parse::<global_hotkey::hotkey::HotKey>().is_ok() {
                                            snippet.hotkey = combo;
                                            snippet.active = true; // capture implies intent to use
                                            self.capturing = None;
                                            out.committed = true;
                                            out.just_activated = Some(i);
                                            break;
                                        }
                                        self.capture_hint = Some("unsupported key");
                                    }
                                }
                                if resp.clicked() {
                                    self.capturing = None; // clicking again cancels
                                }
                            } else {
                                let label = if snippet.hotkey.is_empty() {
                                    egui::RichText::new("click to set").weak().italics()
                                } else if snippet.active {
                                    egui::RichText::new(&snippet.hotkey)
                                } else {
                                    egui::RichText::new(&snippet.hotkey).color(palette.dim)
                                };
                                if hotkey_chip(ui, HOTKEY_W, label)
                                    .on_hover_text("Click, then press the key combo (Esc cancels)")
                                    .clicked()
                                {
                                    self.capturing = Some(i);
                                    self.capture_hint = None; // fresh capture, fresh prompt
                                }
                            }
                            // lost_focus fires on click-away AND on Enter — one signal covers both
                            out.committed |= r_text.lost_focus();
                            // ghost icon button, the production convention for row
                            // utilities: no frame at rest, muted icon; red when aimed at
                            let trash_tint = if self.hovered_delete == Some(i) {
                                palette.danger
                            } else {
                                palette.dim
                            };
                            // white source asset × tint = exact color
                            let trash =
                                egui::Image::new(egui::include_image!("../../assets/trash.svg"))
                                    .fit_to_exact_size(egui::vec2(14.0, 14.0))
                                    .tint(trash_tint);
                            let del_resp = ui
                                .add_sized(
                                    [DELETE_W, 26.0],
                                    egui::Button::image(trash).frame(false),
                                )
                                .on_hover_text("Delete snippet");
                            if del_resp.hovered() {
                                hovered_delete_now = Some(i);
                            }
                            if del_resp.clicked() {
                                delete = Some(i);
                            }
                            ui.end_row();
                        }
                    });
            });

        if let Some(i) = delete {
            snippets.remove(i); // after the closures — the iter_mut borrow is released here
            out.committed = true;
        }
        self.hovered_delete = hovered_delete_now; // next frame's tint — the lag IS the design

        out
    }
}

/// The hotkey cell's button, hand-painted: a stock egui Button strokes its
/// border centered on the rect edge (half a pixel outside), which lands its
/// visible band 1px off the TextEdits beside it no matter how the rect is
/// nudged. This chip paints fill + inside-stroke exactly like an input does,
/// so the row's controls share one pixel band.
fn hotkey_chip(ui: &mut egui::Ui, width: f32, label: egui::RichText) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let v = ui.visuals();
        let w = if response.is_pointer_button_down_on() {
            &v.widgets.active
        } else if response.hovered() {
            &v.widgets.hovered
        } else {
            &v.widgets.inactive
        };
        let (fill, stroke, radius) = (w.weak_bg_fill, w.bg_stroke, w.corner_radius);
        ui.painter().rect_filled(rect, radius, fill);
        ui.painter()
            .rect_stroke(rect, radius, stroke, egui::StrokeKind::Inside);
        let galley = egui::WidgetText::from(label).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            width,
            egui::TextStyle::Button,
        );
        let pos = rect.center() - galley.size() / 2.0;
        let fallback = ui.visuals().widgets.inactive.fg_stroke.color;
        ui.painter().galley(pos, galley, fallback);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Whether a modifier set may anchor a global hotkey. Ctrl or Alt is required;
/// Shift may only ride along — shift+printable IS typing (shift+0 is ')'), and
/// `RegisterHotKey` swallows a registered combo system-wide, so a shift-only
/// bind would make its character impossible to type anywhere.
fn combo_qualifies(modifiers: &egui::Modifiers) -> bool {
    modifiers.ctrl || modifiers.alt
}

/// Modifier keys arrive as their own physical `Event::Key` presses (egui 0.36+).
/// During capture such a press means "chord in progress", never "this is the key".
fn is_modifier_key(key: egui::Key) -> bool {
    use egui::Key as K;
    matches!(
        key,
        K::ShiftLeft
            | K::ShiftRight
            | K::ControlLeft
            | K::ControlRight
            | K::AltLeft
            | K::AltRight
            | K::SuperLeft
            | K::SuperRight
    )
}

/// Build a global-hotkey combo string from egui input. global-hotkey's parser
/// accepts egui's friendly key names ("2", "A", "F5") directly — no mapping
/// table needed; `.parse::<HotKey>()` is the validator.
fn combo_from(modifiers: &egui::Modifiers, key: egui::Key) -> String {
    let mut combo = String::new();
    if modifiers.ctrl {
        combo.push_str("ctrl+");
    }
    if modifiers.alt {
        combo.push_str("alt+");
    }
    if modifiers.shift {
        combo.push_str("shift+");
    }
    combo.push_str(key.name());
    combo
}

#[cfg(test)]
mod tests {
    use super::*;
    use global_hotkey::hotkey::HotKey;

    fn mods(ctrl: bool, alt: bool, shift: bool) -> egui::Modifiers {
        egui::Modifiers { ctrl, alt, shift, ..Default::default() }
    }

    /// The property the whole capture feature rests on: combo strings built from
    /// egui key names are accepted by global-hotkey's parser.
    #[test]
    fn captured_combos_parse_as_hotkeys() {
        let cases = [
            (mods(true, true, false), egui::Key::Num2, "ctrl+alt+2"),
            (mods(true, false, false), egui::Key::A, "ctrl+A"),
            (mods(false, true, true), egui::Key::F5, "alt+shift+F5"),
            (mods(true, false, false), egui::Key::Space, "ctrl+Space"),
            (mods(true, false, true), egui::Key::Minus, "ctrl+shift+Minus"),
        ];
        for (m, key, expected) in cases {
            let combo = combo_from(&m, key);
            assert_eq!(combo, expected);
            combo
                .parse::<HotKey>()
                .unwrap_or_else(|e| panic!("'{combo}' must parse: {e:?}"));
        }
    }

    #[test]
    fn navigation_keys_parse_too() {
        for key in [egui::Key::ArrowDown, egui::Key::ArrowUp, egui::Key::Home, egui::Key::Tab] {
            let combo = combo_from(&mods(true, true, false), key);
            assert!(combo.parse::<HotKey>().is_ok(), "'{combo}' failed to parse");
        }
    }

    /// The regression this pins: pressing bare Ctrl during capture flashed
    /// "unsupported key" — egui 0.36 emits modifier keys as physical presses,
    /// and their names ("ControlLeft") are not combo material.
    #[test]
    fn modifier_presses_are_chord_starts_not_combos() {
        use egui::Key as K;
        let modifier_keys = [
            K::ShiftLeft, K::ShiftRight, K::ControlLeft, K::ControlRight,
            K::AltLeft, K::AltRight, K::SuperLeft, K::SuperRight,
        ];
        for key in modifier_keys {
            assert!(is_modifier_key(key), "{key:?} must be skipped by capture");
        }
        for key in [K::A, K::Num0, K::F5, K::Space, K::Escape] {
            assert!(!is_modifier_key(key), "{key:?} must reach normal capture");
        }
    }

    /// The regression this pins: shift+0 once captured as a hotkey, which made
    /// ')' untypeable system-wide. Shift never qualifies a combo on its own.
    #[test]
    fn shift_alone_never_qualifies() {
        assert!(!combo_qualifies(&mods(false, false, true)), "shift-only is typing");
        assert!(!combo_qualifies(&mods(false, false, false)), "bare keys are typing");
        assert!(combo_qualifies(&mods(true, false, false)));
        assert!(combo_qualifies(&mods(false, true, false)));
        assert!(combo_qualifies(&mods(true, false, true)), "shift may ride along");
    }
}
