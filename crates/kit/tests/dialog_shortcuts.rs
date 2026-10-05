mod common;
use gpui_kit::component::{
    WindowExt,
    button::Button,
    dialog::{DialogClose, DialogFooter},
    input::{Input, InputState},
};
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, Entity, InputEvent, KeyDownEvent, Keystroke, TestAppContext, Window, div,
    prelude::*, px, size,
};
use pretty_assertions::assert_eq;
use std::{cell::Cell, rc::Rc};

struct Workspace {
    focus: gpui_kit::FocusHandle,
    draft: Entity<InputState>,
    closed: Rc<Cell<usize>>,
}
impl Render for Workspace {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let draft = self.draft.clone();
        let closed = self.closed.clone();
        div()
            .id("workspace")
            .test_support()
            .track_focus(&self.focus)
            .size_full()
            .p_4()
            .child(
                Button::new("open")
                    .label("Open…")
                    .on_click(move |_, window, cx| {
                        let draft = draft.clone();
                        let closed = closed.clone();
                        window.open_dialog(cx, move |dialog, _, _| {
                            let closed = closed.clone();
                            dialog
                                .title("Shortcuts")
                                .capture_key_down(|event, window, cx| {
                                    let modifiers = event.keystroke.modifiers;
                                    if event.keystroke.key == "k"
                                        && modifiers.control
                                        && !modifiers.platform
                                        && !modifiers.alt
                                        && !modifiers.shift
                                    {
                                        if !event.is_held {
                                            window.dispatch_action(
                                                Box::new(gpui_kit::base::actions::Cancel),
                                                cx,
                                            );
                                        }
                                        cx.stop_propagation();
                                    }
                                })
                                .child(Input::new(&draft).id("draft"))
                                .child(Button::new("nested").label("Nested…").on_click(
                                    |_, window, cx| {
                                        window.open_dialog(cx, |dialog, _, _| {
                                            dialog.title("Nested").footer(
                                                DialogFooter::new().child(
                                                    DialogClose::new().trigger(|button| {
                                                        div()
                                                            .id("nested-close")
                                                            .test_support()
                                                            .child(button.label("Done"))
                                                    }),
                                                ),
                                            )
                                        });
                                    },
                                ))
                                .footer(
                                    DialogFooter::new()
                                        .child(Button::new("footer-focus").label("Keep open"))
                                        .child(
                                            DialogClose::new()
                                                .trigger(|button| button.label("Done")),
                                        ),
                                )
                                .on_close(move |_, _, _| closed.set(closed.get() + 1))
                        });
                    }),
            )
    }
}

#[gpui_kit::test]
async fn dialog_capture_covers_initial_input_and_footer_focus_and_preserves_editing(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let closed = Rc::new(Cell::new(0));
    let (handle, _) = common::open_window(cx, Some(size(px(800.), px(700.))), |window, cx| {
        let view = cx.new(|cx| Workspace {
            focus: cx.focus_handle(),
            draft: cx.new(|cx| InputState::new(window, cx)),
            closed: closed.clone(),
        });
        view.read(cx).focus.clone().focus(window, cx);
        view
    });
    for (ix, focus) in [None, Some("draft"), Some("footer-focus")]
        .into_iter()
        .enumerate()
    {
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            window.click("open", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            assert!(window.has_active_dialog(cx));
            if let Some(focus) = focus {
                if focus == "footer-focus" {
                    // Buttons retain pointer focus elsewhere; keyboard traversal owns their focus.
                    for _ in 0..8 {
                        window.press("tab", cx);
                        if window.find(focus).focused() == Some(true) {
                            break;
                        }
                    }
                } else {
                    window.click(focus, cx);
                }
                assert_eq!(window.find(focus).focused(), Some(true));
                if focus == "draft" {
                    window.input("a", cx);
                    window.press("backspace", cx);
                    assert_eq!(window.find("draft").value(), Some(""));
                }
            }
            // Held KEY_DOWN consumes this exact shortcut without toggling twice.
            window.dispatch_event(
                KeyDownEvent {
                    keystroke: Keystroke::parse("ctrl-k").unwrap(),
                    is_held: true,
                    prefer_character_input: false,
                }
                .to_platform_input(),
                cx,
            );
            window.press("ctrl-shift-k", cx);
            assert!(window.has_active_dialog(cx));
            window.press("ctrl-k", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            assert!(!window.has_active_dialog(cx));
            assert_eq!(window.find("workspace").focused(), Some(true));
        })
        .unwrap();
        assert_eq!(closed.get(), ix + 1);
    }
}

#[gpui_kit::test]
async fn underlying_dialog_capture_cannot_dismiss_a_nested_modal(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let closed = Rc::new(Cell::new(0));
    let (handle, _) = common::open_window(cx, Some(size(px(800.), px(700.))), |window, cx| {
        let view = cx.new(|cx| Workspace {
            focus: cx.focus_handle(),
            draft: cx.new(|cx| InputState::new(window, cx)),
            closed: closed.clone(),
        });
        view.read(cx).focus.clone().focus(window, cx);
        view
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("open", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click("nested", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.press("ctrl-k", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("nested-close").visible());
        assert_eq!(closed.get(), 0);
        // The native close control dismisses only its own modal.
        window.click("nested-close", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert!(window.has_active_dialog(cx));
        window.press("ctrl-k", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        assert!(!window.has_active_dialog(cx));
        assert_eq!(closed.get(), 1);
    })
    .unwrap();
}
