#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::get_serial_list;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::get_serial_list;

#[derive(Debug)]
pub struct UsbInfo {
    pub vid: String,
    pub pid: String,
}

#[derive(Debug)]
pub struct SerialInfo {
    pub name: String,
    pub vendor: Option<String>,
    pub product: Option<String>,
    pub driver: Option<String>,
    pub usb_info: Option<UsbInfo>,
}
