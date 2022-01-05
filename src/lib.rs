#[cfg(target_os = "windows")]
mod win;

#[cfg(target_os = "windows")]
pub use win::get_serial_list;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::get_serial_list;

#[derive(Debug)]
/// usb information of serial port
pub struct UsbInfo {
    pub vid: String,
    pub pid: String,
}

#[derive(Debug)]
/// serial port informations
pub struct SerialInfo {
    /// serial port name
    pub name: String,
    /// vendor info
    pub vendor: Option<String>,
    /// product info
    pub product: Option<String>,
    /// linux only, driver name of current serial port
    pub driver: Option<String>,
    /// usb serial port only, vid and pid provided
    pub usb_info: Option<UsbInfo>,
}
