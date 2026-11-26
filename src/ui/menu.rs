//! Styled popovers, menus, context menus, menu items, and separators backed by `theme()`.

use nsh_base::{
    context_menu, menu, menu_item, menu_separator, popover, theme, ContextMenuElement,
    MenuElement, MenuItemElement, MenuSeparatorElement, PopoverElement,
};
use nsh_core::element::StyleRefinement;
use nsh_core::prelude::*;

use super::icons::{self, svg_icon};
use super::{radius, MOTION_INSTANT_MS};

fn popover_surface_style(s: StyleRefinement) -> StyleRefinement {
    s.rounded(radius)
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().popover)
        .text_color(move || theme().popover_foreground)
        .shadow_md()
        .transition_all()
        .duration(MOTION_INSTANT_MS)
}

fn menu_surface_style(s: StyleRefinement) -> StyleRefinement {
    popover_surface_style(s).min_w(px(160.0)).p_1()
}

fn menu_item_base(on_select: impl Fn() + 'static) -> MenuItemElement {
    menu_item()
        .on_select(on_select)
        .h(px(30.0))
        .px_2()
        .flex_row()
        .items_center()
        .gap_2()
        .rounded(px(6.0))
        .hover(|s| s.bg(move || theme().accent))
        .focus(|s| s.bg(move || theme().accent))
        .active(|s| s.bg(move || theme().muted))
        .disabled_style(|s| s.opacity(0.4))
}

/// A popover anchored to a trigger with Bento's semantic popover surface, border, radius,
/// shadow, and subtle fade/scale entrance and exit transitions.
pub fn styled_popover() -> PopoverElement {
    popover()
        .offset(px(6.0))
        .margin(px(8.0))
        .popup_style(|s| popover_surface_style(s).p_3())
        .popup_starting_style(|s| s.opacity(0.0).scale(0.96))
        .popup_ending_style(|s| s.opacity(0.0).scale(0.96))
}

/// A right-click / long-press context menu styled with Bento's popover surface tokens.
pub fn styled_context_menu() -> ContextMenuElement {
    context_menu()
        .offset(px(4.0))
        .margin(px(8.0))
        .popup_style(menu_surface_style)
        .popup_starting_style(|s| s.opacity(0.0).scale(0.96))
        .popup_ending_style(|s| s.opacity(0.0).scale(0.96))
}

/// A trigger-opened dropdown menu styled with Bento's popover surface tokens.
pub fn styled_menu() -> MenuElement {
    menu()
        .offset(px(4.0))
        .margin(px(8.0))
        .popup_style(menu_surface_style)
        .popup_starting_style(|s| s.opacity(0.0).scale(0.96))
        .popup_ending_style(|s| s.opacity(0.0).scale(0.96))
}

/// A styled menu item with hover/focus accent background and `popover_foreground` text.
pub fn styled_menu_item(
    label: impl IntoReactive<SharedString>,
    on_select: impl Fn() + 'static,
) -> MenuItemElement {
    menu_item_base(on_select).child(
        text(label)
            .text_xs()
            .text_color(move || theme().popover_foreground),
    )
}

/// A checkable styled menu item displaying a leading checkmark indicator when `checked()` is true.
pub fn styled_menu_item_checked(
    checked: impl Fn() -> bool + Copy + 'static,
    label: impl IntoReactive<SharedString>,
    on_select: impl Fn() + 'static,
) -> MenuItemElement {
    menu_item_base(on_select)
        .checked(move || Some(checked()))
        .indicator_style(|s| {
            s.w(px(16.0))
                .h(px(16.0))
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .opacity(0.0)
                .checked(|s| s.opacity(1.0))
        })
        .indicator(svg_icon(icons::CHECK, 13.0, move || theme().primary))
        .child(
            text(label)
                .text_xs()
                .text_color(move || theme().popover_foreground),
        )
}

/// A 1pt horizontal separator between groups in [`styled_menu`] or [`styled_context_menu`].
pub fn styled_menu_separator() -> MenuSeparatorElement {
    menu_separator()
        .h(px(1.0))
        .my_1()
        .mx(px(4.0))
        .bg(move || theme().border)
}
