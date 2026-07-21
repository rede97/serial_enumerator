//! Parser for Windows device instance IDs.
//!
//! USB serial adapters encode their VID/PID in the device instance ID:
//!
//! - Standard USB: `USB\VID_1A86&PID_7523\7&139F9FFA&0&2`
//! - Composite (multi-interface) USB: `USB\VID_0483&PID_5740&MI_00\...`
//! - FTDI devices use `+` as separator: `FTDIBUS\VID_0403+PID_6001+SERIAL\0000`
//!
//! Non-USB devices (e.g. `ACPI\PNP0501\1`) yield `None`.

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while_m_n};
use nom::sequence::{preceded, separated_pair};
use nom::{AsChar, IResult};

fn usb_prefix_parser(s: &str) -> IResult<&str, &str> {
    alt((tag("USB"), tag("FTDIBUS")))(s)
}

fn usbid_parser(s: &str) -> IResult<&str, &str> {
    take_while_m_n(4, 4, |c: char| c.is_hex_digit())(s)
}

/// Standard USB IDs separate fields with "&", FTDI IDs use "+".
fn separator_parser(s: &str) -> IResult<&str, &str> {
    alt((tag("&"), tag("+")))(s)
}

/// Parses "\VID_xxxx<sep>PID_xxxx". Anything after the PID
/// ("&MI_xx", "+SERIAL", "\...") is left unparsed.
fn vid_pid_parser(s: &str) -> IResult<&str, (&str, &str)> {
    preceded(
        tag("\\VID_"),
        separated_pair(
            usbid_parser,
            separator_parser,
            preceded(tag("PID_"), usbid_parser),
        ),
    )(s)
}

fn device_id_parser(s: &str) -> IResult<&str, (&str, &str)> {
    preceded(usb_prefix_parser, vid_pid_parser)(s)
}

/// Extract `(vid, pid)` from a Windows device instance ID.
/// Returns `None` for non-USB devices.
pub fn parse_device_id(device_id: &str) -> Option<(String, String)> {
    match device_id_parser(device_id) {
        Ok((_, (vid, pid))) => Some((vid.into(), pid.into())),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_usb_id() {
        let result = parse_device_id(r"USB\VID_1A86&PID_7523\7&139F9FFA&0&2").unwrap();
        assert_eq!(result, ("1A86".into(), "7523".into()));
    }

    #[test]
    fn test_ftdi_plus_separator() {
        let result = parse_device_id(r"FTDIBUS\VID_0403+PID_6001+AL02J4QAA\0000").unwrap();
        assert_eq!(result, ("0403".into(), "6001".into()));
    }

    #[test]
    fn test_composite_device_with_mi() {
        let result = parse_device_id(r"USB\VID_0483&PID_5740&MI_00\7&2A1B3C4D&0&0").unwrap();
        assert_eq!(result, ("0483".into(), "5740".into()));
    }

    #[test]
    fn test_id_without_trailing_path() {
        let result = parse_device_id(r"USB\VID_2341&PID_0043").unwrap();
        assert_eq!(result, ("2341".into(), "0043".into()));
    }

    #[test]
    fn test_non_usb_devices() {
        // built-in serial port
        assert_eq!(parse_device_id(r"ACPI\PNP0501\1"), None);
        // PCI serial card
        assert_eq!(
            parse_device_id(r"PCI\VEN_8086&DEV_9D3D&SUBSYS_207017AA\3&11583659&0&B0"),
            None
        );
        // Bluetooth SPP port
        assert_eq!(
            parse_device_id(r"BTHENUM\{00001101-0000-1000-8000-00805F9B34FB}\8&2A1B3C4D&0&0"),
            None
        );
    }
}
