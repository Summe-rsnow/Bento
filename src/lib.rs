//! Bento: shared Native Shell (NSH) UI component library and desktop shell
//! infrastructure for Wonton, Sundae, and companion desktop tools.

pub mod drag_list;
pub mod shell;
pub mod ui;
pub mod updater;

pub use updater::AppMeta;

/// The standard Bento Light/Dark semantic theme definition (`theme.json`).
pub const DEFAULT_THEME_JSON: &str = include_str!("../theme.json");
