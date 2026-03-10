/// Axis-aligned rectangle defined by origin (x, y) and size (w, h).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    /// Horizontal origin.
    pub x: f32,
    /// Vertical origin.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Compute the area (w × h).
    pub fn area(self) -> f32 {
        self.w * self.h
    }

    /// Return the center point as `(x, y)`.
    pub fn center(self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// Returns true when the point lies inside. Lower bound inclusive, upper exclusive.
    pub fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    /// Returns true when the two rects share positive overlap area (touching edges don't count).
    pub fn intersects(self, other: Rect) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    /// Linear interpolation between `self` and `other` across all four fields.
    pub fn lerp(self, other: Rect, t: f32) -> Rect {
        Rect {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            w: self.w + (other.w - self.w) * t,
            h: self.h + (other.h - self.h) * t,
        }
    }
}
