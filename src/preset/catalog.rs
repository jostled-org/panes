/// Whether a preset accepts a dynamic list of panels or fixed named slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelInputKind {
    /// Accepts an arbitrary number of panels (e.g. `master_stack(["a", "b", "c"])`).
    DynamicList,
    /// Accepts a fixed set of named slots (e.g. `sidebar("nav", "content")`).
    FixedSlots,
}

/// Metadata about a built-in preset layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetInfo {
    /// Kebab-case name matching the TOML `strategy` field.
    pub name: &'static str,
    /// Whether the preset takes a dynamic list or fixed slots.
    pub input: PanelInputKind,
    /// One-line description of the preset.
    pub description: &'static str,
}

pub(crate) const PRESETS: [PresetInfo; 13] = [
    PresetInfo {
        name: "centered-master",
        input: PanelInputKind::DynamicList,
        description: "Master pane centered, remaining panes split left and right",
    },
    PresetInfo {
        name: "dashboard",
        input: PanelInputKind::DynamicList,
        description: "CSS Grid with per-card column spans, responsive reflow, and full-width cards",
    },
    PresetInfo {
        name: "deck",
        input: PanelInputKind::DynamicList,
        description: "Master pane with a single visible card in the stack",
    },
    PresetInfo {
        name: "dwindle",
        input: PanelInputKind::DynamicList,
        description: "Recursive split alternating horizontal and vertical",
    },
    PresetInfo {
        name: "holy-grail",
        input: PanelInputKind::FixedSlots,
        description: "Header, footer, left sidebar, main content, right sidebar",
    },
    PresetInfo {
        name: "master-stack",
        input: PanelInputKind::DynamicList,
        description: "One primary pane on the left, remaining panes stacked on the right",
    },
    PresetInfo {
        name: "monocle",
        input: PanelInputKind::DynamicList,
        description: "Single fullscreen pane, others hidden",
    },
    PresetInfo {
        name: "scrollable",
        input: PanelInputKind::DynamicList,
        description: "Horizontal strip of fixed-width columns exceeding viewport",
    },
    PresetInfo {
        name: "sidebar",
        input: PanelInputKind::FixedSlots,
        description: "Fixed-width sidebar with a growing content area",
    },
    PresetInfo {
        name: "spiral",
        input: PanelInputKind::DynamicList,
        description: "Like dwindle but reverses child order on even-depth levels",
    },
    PresetInfo {
        name: "split",
        input: PanelInputKind::FixedSlots,
        description: "Two panels, horizontal or vertical",
    },
    PresetInfo {
        name: "stacked",
        input: PanelInputKind::DynamicList,
        description: "Vertical title bars over a single visible content pane",
    },
    PresetInfo {
        name: "tabbed",
        input: PanelInputKind::DynamicList,
        description: "Tab header bar over a single visible content pane",
    },
];
