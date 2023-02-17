use crate::{SerialInfo};
use std::fs;
use std::vec::Vec;

fn get_device_files() -> impl Iterator<Item=fs::DirEntry> {
    const DEV_PATH: &str = "/dev";
    let read_dir = fs::read_dir(DEV_PATH).expect("could not list the /dev directory");
    return read_dir.filter_map(|entry| entry.ok());
}

/// enumerate all avaliable serial port
pub fn get_serial_list() -> Vec<SerialInfo> {
    let devices = get_device_files();

    let paths = devices.map(|d| d.path()
                                 .to_str()
                                 .expect(format!("could not convert file name to string: {:?}", d).as_str())
                                 .to_string());
    
    let serial_devices = paths.filter(|path| path.starts_with("/dev/cu"));

    return serial_devices.map(|d| SerialInfo {
        name: d.to_string(),
        driver: None,
        vendor: None,
        product: None,
        usb_info: None
    }).collect();
}