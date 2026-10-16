#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use gpui::{
    App, Context, Font, FontFeatures, IntoElement, Modifiers, MouseButton, MouseDownEvent, Render,
    RenderOnce, TestAppContext, Window, div, prelude::*,
};
use gpui_component::{
    button::Button,
    menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem},
};
use std::{cell::RefCell, rc::Rc};

#[derive(IntoElement)]
struct ObserveFont(Rc<RefCell<Vec<Font>>>);
impl RenderOnce for ObserveFont {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        self.0.borrow_mut().push(window.text_style().font());
        div().child("Menu text")
    }
}
struct Host {
    dropdown: bool,
    observed: Rc<RefCell<Vec<Font>>>,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let observed = self.observed.clone();
        let menu = move |menu: PopupMenu, _: &mut Window, _: &mut Context<PopupMenu>| {
            let observed = observed.clone();
            menu.item(PopupMenuItem::element(move |_, _| {
                ObserveFont(observed.clone())
            }))
        };
        let trigger = if self.dropdown {
            Button::new("menu-trigger")
                .w_full()
                .label("Open")
                .dropdown_menu(menu)
                .into_any_element()
        } else {
            div()
                .id("context-trigger")
                .w_32()
                .h_8()
                .child("Open")
                .context_menu(menu)
                .into_any_element()
        };
        div()
            .font_family("Origin font")
            .font_features(FontFeatures(std::sync::Arc::new(vec![("test".into(), 1)])))
            .child(
                div()
                    .id("trigger-bounds")
                    .debug_selector(|| "trigger-bounds".into())
                    .w_32()
                    .h_8()
                    .child(trigger),
            )
    }
}

#[gpui::test]
fn deferred_menus_preserve_the_trigger_font_role(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    for dropdown in [false, true] {
        let observed = Rc::new(RefCell::new(Vec::new()));
        let (_, visual) = cx.add_window_view({
            let observed = observed.clone();
            move |_, _| Host { dropdown, observed }
        });
        visual.update(|window, cx| window.draw(cx).clear(cx));
        let target = visual.debug_bounds("trigger-bounds").unwrap().center();
        if dropdown {
            visual.simulate_click(target, Modifiers::default());
        } else {
            visual.simulate_event(MouseDownEvent {
                button: MouseButton::Right,
                position: target,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            });
        }
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        let fonts = observed.borrow();
        assert!(!fonts.is_empty(), "menu did not open: dropdown={dropdown}");
        assert!(
            fonts
                .iter()
                .all(|font| font.family.as_ref() == "Origin font"
                    && font
                        .features
                        .0
                        .iter()
                        .any(|(tag, value)| tag == "test" && *value == 1)),
            "menu lost its role: {fonts:?}"
        );
    }
}
