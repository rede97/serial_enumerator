# serial_enumerator

[![Crates.io](https://img.shields.io/crates/v/serial_enumerator)](https://crates.io/crates/serial_enumerator)
[![Crates.io Downloads](https://img.shields.io/crates/d/serial_enumerator)](https://crates.io/crates/serial_enumerator)
[![Docs.rs](https://docs.rs/serial_enumerator/badge.svg)](https://docs.rs/serial_enumerator)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://github.com/rede97/serial_enumerator/actions/workflows/rust.yml/badge.svg)](https://github.com/rede97/serial_enumerator/actions/workflows/rust.yml)

A serial port enumeration library written in Rust. It lists available serial ports with vendor, product, and USB VID/PID information.

- Cross-platform: **Linux**, **Windows**, **macOS**
- Filters out non-functional ports so you only see what can actually communicate
- On Linux, distinguishes valid ports from false positives via the `valid` field — unlike tools that dump every TTY blindly

## Quick Start

```rust
use serial_enumerator::get_serial_list;

fn main() {
    for port in get_serial_list() {
        println!("{}  valid={}  {:?}", port.name, port.valid, port.usb_info);
    }
}
```

## API

```rust
pub fn get_serial_list() -> Vec<SerialInfo>
```

Returns all available serial ports on the current platform.

### SerialInfo

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Port name (e.g. `COM3`, `/dev/ttyUSB0`) |
| `valid` | `bool` | Whether the port is confirmed usable for communication |
| `vendor` | `Option<String>` | Manufacturer / vendor name |
| `product` | `Option<String>` | Product description |
| `driver` | `Option<String>` | Kernel driver name (Linux only) |
| `usb_info` | `Option<UsbInfo>` | USB VID/PID if it's a USB serial device |

### UsbInfo

| Field | Type | Description |
|---|---|---|
| `vid` | `String` | Vendor ID (hex, e.g. `"1A86"`) |
| `pid` | `String` | Product ID (hex, e.g. `"7523"`) |

## CLI Tool: `lser`

This crate also ships a built-in CLI tool for listing serial ports from the terminal:

```bash
cargo install serial_enumerator
lser
```

Or run it directly:

```bash
cargo run --bin lser
```

Example output (Linux):

```
SerialInfo { name: "/dev/ttyS0", valid: true, vendor: Some("pnp"), product: Some("PNP0501"), driver: Some("serial"), usb_info: None }
SerialInfo { name: "/dev/ttyUSB0", valid: true, vendor: Some("FTDI"), product: Some("Dual RS232-HS:00"), driver: Some("usbserial"), usb_info: Some(UsbInfo { vid: "0403", pid: "6010" }) }
```

Example output (Windows):

```
SerialInfo { name: "COM4", valid: true, vendor: Some("wch.cn"), product: Some("USB-SERIAL CH340"), driver: None, usb_info: Some(UsbInfo { vid: "1A86", pid: "7523" }) }
```

## Validity Behavior by Platform

| Platform | Filtering | `valid` field |
|---|---|---|
| **Windows** | Only `COM`-prefixed ports are returned | Always `true` |
| **Linux** | All TTY devices with recognized serial prefixes are returned | `true` if probe confirms USB descriptors, device-tree node, or PNP ID; `false` otherwise |
| **macOS** | All IOKit serial BSD services are returned | Always `true` |

## License

MIT
