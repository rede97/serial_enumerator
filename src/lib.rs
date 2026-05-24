//! # serial_enumerator
//!
//! [![Crates.io](https://img.shields.io/crates/v/serial_enumerator)](https://crates.io/crates/serial_enumerator)
//! [![Crates.io Downloads](https://img.shields.io/crates/d/serial_enumerator)](https://crates.io/crates/serial_enumerator)
//! [![Docs.rs](https://docs.rs/serial_enumerator/badge.svg)](https://docs.rs/serial_enumerator)
//! [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
//! [![Build Status](https://github.com/rede97/serial_enumerator/actions/workflows/rust.yml/badge.svg)](https://github.com/rede97/serial_enumerator/actions/workflows/rust.yml)
//!
//! A cross-platform serial port enumeration library for Linux, Windows, and macOS.
//! Returns available serial ports with vendor, product, driver, and USB VID/PID
//! information.
//!
//! Unlike tools that blindly list every TTY device, this library validates each port
//! and exposes a [`valid`](SerialInfo::valid) field so you can distinguish functional
//! ports from false positives.
//!
//! ## Quick Start
//!
//! ```rust
//! use serial_enumerator::get_serial_list;
//!
//! for port in get_serial_list() {
//!     println!("{}  valid={}", port.name, port.valid);
//! }
//! ```
//!
//! ## CLI Tool
//!
//! A command-line tool [`lser`](https://crates.io/crates/lser) is included.
//! Install with:
//!
//! ```bash
//! cargo install serial_enumerator
//! lser
//! ```
//!
//! ## Example Output
//!
//! **Linux:**
//!
//! ```text
//! SerialInfo { name: "/dev/ttyS0", valid: true, vendor: Some("pnp"), product: Some("PNP0501"), driver: Some("serial"), usb_info: None }
//! SerialInfo { name: "/dev/ttyUSB0", valid: true, vendor: Some("FTDI"), product: Some("Dual RS232-HS:00"), driver: Some("usbserial"), usb_info: Some(UsbInfo { vid: "0403", pid: "6010" }) }
//! ```
//!
//! **Windows:**
//!
//! ```text
//! SerialInfo { name: "COM4", valid: true, vendor: Some("wch.cn"), product: Some("USB-SERIAL CH340"), driver: None, usb_info: Some(UsbInfo { vid: "1A86", pid: "7523" }) }
//! ```

#[cfg(target_os = "windows")]
mod win;

#[cfg(target_os = "windows")]
pub use win::get_serial_list;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::get_serial_list;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::get_serial_list;

/// USB identification for a serial device.
#[derive(Debug)]
pub struct UsbInfo {
    /// Vendor ID as a 4-character hex string (e.g. `"1A86"`).
    pub vid: String,
    /// Product ID as a 4-character hex string (e.g. `"7523"`).
    pub pid: String,
}

/// Information about an enumerated serial port.
///
/// Returned by [`get_serial_list`].
#[derive(Debug)]
pub struct SerialInfo {
    /// Port name — `"COM3"` on Windows, `"/dev/ttyUSB0"` on Linux/macOS.
    pub name: String,
    /// Whether the port is confirmed usable for serial communication.
    ///
    /// On Linux, this reflects whether the probe found USB descriptors,
    /// a device-tree node, or a PNP ID. On Windows and macOS it is always
    /// `true` for returned ports.
    pub valid: bool,
    /// Manufacturer or vendor name (e.g. `"FTDI"`, `"wch.cn"`).
    pub vendor: Option<String>,
    /// Product description string.
    pub product: Option<String>,
    /// Kernel driver name (Linux only; `None` on other platforms).
    pub driver: Option<String>,
    /// USB VID/PID if the port is a USB-serial adapter.
    pub usb_info: Option<UsbInfo>,
}
