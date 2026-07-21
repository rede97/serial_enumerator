//! Smoke tests: enumeration must not panic even on machines without
//! any serial hardware (e.g. CI runners). When ports do exist, their
//! fields are validated against the documented invariants.

use serial_enumerator::get_serial_list;

#[test]
fn get_serial_list_does_not_panic() {
    let ports = get_serial_list();
    for port in &ports {
        assert!(!port.name.is_empty(), "port name must not be empty");
    }
}

#[test]
fn usb_info_follows_format_contract() {
    for port in get_serial_list() {
        if let Some(usb) = &port.usb_info {
            for (field, value) in [("vid", &usb.vid), ("pid", &usb.pid)] {
                assert_eq!(
                    value.len(),
                    4,
                    "{} of {} must be a 4-character hex string, got {:?}",
                    field,
                    port.name,
                    value
                );
                assert!(
                    value.chars().all(|c| c.is_ascii_hexdigit()),
                    "{} of {} must be hex, got {:?}",
                    field,
                    port.name,
                    value
                );
            }
        }
    }
}
