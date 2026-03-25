//! Convert panes layouts into `egui::Rect` values.

panes::impl_adapter! {
    rect: egui::Rect,
    origin: egui::Pos2,
    convert_fn: |r: &panes::Rect| {
        egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h))
    },
    convert_at_fn: |r: &panes::Rect, origin: egui::Pos2| {
        egui::Rect::from_min_size(
            egui::pos2(r.x + origin.x, r.y + origin.y),
            egui::vec2(r.w, r.h),
        )
    },
}
