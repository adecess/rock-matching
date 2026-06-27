use rock_matching_engine::Level;
use std::fmt::Write;

pub(crate) fn format_levels(levels: &[Level]) -> String {
    let mut formatted_levels = String::new();
    for (i, level) in levels.iter().enumerate() {
        if i > 0 {
            formatted_levels.push(' ');
        }
        write!(
            &mut formatted_levels,
            "{:?}x{:?}",
            level.price.0, level.quantity.0
        )
        .unwrap();
    }

    formatted_levels
}
