use panes::Rect;

#[test]
fn rect_area_computes_width_times_height() {
    let r = Rect {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 5.0,
    };
    assert_eq!(r.area(), 50.0);
}

#[test]
fn rect_center_returns_midpoint() {
    let r = Rect {
        x: 10.0,
        y: 20.0,
        w: 40.0,
        h: 60.0,
    };
    assert_eq!(r.center(), (30.0, 50.0));
}

#[test]
fn rect_contains_point_inside() {
    let r = Rect {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    assert!(r.contains(5.0, 5.0));
    assert!(r.contains(0.0, 0.0)); // edge inclusive
    assert!(!r.contains(10.0, 10.0)); // exclusive upper bound
    assert!(!r.contains(15.0, 5.0));
}

#[test]
fn rect_intersects_overlapping() {
    let a = Rect {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    let b = Rect {
        x: 5.0,
        y: 5.0,
        w: 10.0,
        h: 10.0,
    };
    assert!(a.intersects(b));

    let c = Rect {
        x: 20.0,
        y: 20.0,
        w: 5.0,
        h: 5.0,
    };
    assert!(!a.intersects(c));

    // Edge-adjacent rects (shared edge, zero overlap)
    let d = Rect {
        x: 10.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    assert!(!a.intersects(d));
}

#[test]
fn rect_lerp_at_boundaries() {
    let a = Rect {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    let b = Rect {
        x: 10.0,
        y: 10.0,
        w: 20.0,
        h: 20.0,
    };
    assert_eq!(a.lerp(b, 0.0), a);
    assert_eq!(a.lerp(b, 1.0), b);
}

#[test]
fn rect_lerp_at_midpoint() {
    let a = Rect {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    let b = Rect {
        x: 10.0,
        y: 10.0,
        w: 30.0,
        h: 30.0,
    };
    assert_eq!(
        a.lerp(b, 0.5),
        Rect {
            x: 5.0,
            y: 5.0,
            w: 20.0,
            h: 20.0
        }
    );
}

#[test]
fn rect_zero_dimensions() {
    let r = Rect {
        x: 5.0,
        y: 5.0,
        w: 0.0,
        h: 0.0,
    };
    assert_eq!(r.area(), 0.0);
    assert!(!r.contains(5.0, 5.0));
    assert_eq!(r.center(), (5.0, 5.0));
}

#[test]
fn rect_default_is_zero() {
    assert_eq!(
        Rect::default(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0
        },
    );
}
