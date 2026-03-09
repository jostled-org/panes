#[cfg(feature = "toml")]
use panes::TomlError;
use panes::{ConstraintError, PaneError, PanelId};

#[test]
fn pane_error_panel_not_found_displays_id() {
    let err = PaneError::PanelNotFound(PanelId::from_raw(42));
    let msg = err.to_string();
    assert!(msg.contains("42"), "expected '42' in: {msg}");
}

#[test]
fn pane_error_invalid_constraint_displays_reason() {
    let err = PaneError::InvalidConstraint(ConstraintError::IsNegative("grow"));
    let msg = err.to_string();
    assert!(msg.contains("grow"), "expected 'grow' in: {msg}");
    assert!(msg.contains("negative"), "expected 'negative' in: {msg}");
}

#[test]
fn pane_error_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PaneError>();
}

#[cfg(feature = "toml")]
#[test]
fn toml_error_unknown_strategy_displays_name() {
    let err = TomlError::UnknownStrategy("foobar".into());
    let msg = err.to_string();
    assert!(msg.contains("foobar"), "expected 'foobar' in: {msg}");
}

#[cfg(feature = "toml")]
#[test]
fn toml_error_missing_field_displays_field_name() {
    let err = TomlError::MissingField("panels".into());
    let msg = err.to_string();
    assert!(msg.contains("panels"), "expected 'panels' in: {msg}");
}

#[cfg(feature = "toml")]
#[test]
fn toml_error_invalid_value_displays_field_and_reason() {
    let err = TomlError::InvalidValue {
        field: "gap".into(),
        reason: "must be non-negative".into(),
    };
    let msg = err.to_string();
    assert!(msg.contains("gap"), "expected 'gap' in: {msg}");
    assert!(
        msg.contains("non-negative"),
        "expected 'non-negative' in: {msg}"
    );
}
