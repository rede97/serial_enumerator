use crate::device_id;
use crate::{SerialInfo, UsbInfo};
use std::mem::size_of;
use windows::core::GUID;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiClassGuidsFromNameA, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsA, SetupDiGetDeviceInstanceIdA, SetupDiGetDeviceRegistryPropertyW,
    SetupDiOpenDevRegKey, DICS_FLAG_GLOBAL, DIGCF_PRESENT, DIREG_DEV, HDEVINFO, SPDRP_DEVICEDESC,
    SPDRP_MFG, SP_DEVINFO_DATA,
};
use windows::Win32::System::Registry::{RegCloseKey, RegQueryValueExA, KEY_READ};

fn get_device_class_guids_form_serial() -> Option<Vec<GUID>> {
    const DEVICE_NAME: ::windows::core::PCSTR = ::windows::core::s!("Ports");
    let mut required_size: u32 = 0;
    unsafe {
        // Probe call: fails with ERROR_INSUFFICIENT_BUFFER but fills
        // required_size — the failure is expected, do not abort on it.
        let _ = SetupDiClassGuidsFromNameA(DEVICE_NAME, &mut [], &mut required_size);
    }
    let mut guids = vec![GUID::zeroed(); required_size as usize];
    let status = unsafe { SetupDiClassGuidsFromNameA(DEVICE_NAME, &mut guids, &mut required_size) };

    if status.is_ok() {
        return Some(guids);
    } else {
        return None;
    }
}

unsafe fn get_port_name_from_dev_info(
    dev_set: HDEVINFO,
    dev_inf: &SP_DEVINFO_DATA,
) -> Option<String> {
    let hkey =
        SetupDiOpenDevRegKey(dev_set, dev_inf, DICS_FLAG_GLOBAL, 0, DIREG_DEV, KEY_READ.0).ok()?;
    let mut name_size: u32 = 1024;
    let mut ltype = windows::Win32::System::Registry::REG_VALUE_TYPE(0);
    let mut buffer = vec![0u8; name_size as usize];
    let result = RegQueryValueExA(
        hkey,
        ::windows::core::s!("PortName"),
        None,
        Some(&mut ltype),
        Some(buffer.as_mut_ptr()),
        Some(&mut name_size),
    );
    RegCloseKey(hkey).ok()?;
    if result.is_ok() {
        buffer.truncate(name_size as usize - 1);
        let port_name = String::from_utf8(buffer).unwrap();
        return Some(port_name);
    }
    return None;
}

unsafe fn get_usb_info(dev_set: HDEVINFO, dev_inf: &SP_DEVINFO_DATA) -> Option<UsbInfo> {
    let mut id_size: u32 = 1024;
    let mut buffer = vec![0u8; id_size as usize];
    if SetupDiGetDeviceInstanceIdA(dev_set, dev_inf, Some(&mut buffer), Some(&mut id_size)).is_ok()
    {
        buffer.truncate(id_size as usize - 1);
        let device_id = String::from_utf8(buffer).unwrap();
        return device_id::parse_device_id(device_id.as_str())
            .and_then(|(vid, pid)| Some(UsbInfo { vid, pid }));
    }
    return None;
}

fn utf16_to_utf8(utf16: &[u16]) -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::prelude::*;
    let wstr = OsString::from_wide(utf16);
    return wstr
        .to_str()
        .and_then(|s| s.split_once('\0'))
        .and_then(|(s, _)| Some(String::from(s)));
}

unsafe fn get_serial_property(
    dev_set: HDEVINFO,
    dev_inf: &SP_DEVINFO_DATA,
    property: u32,
) -> Option<String> {
    let mut property_size: u32 = 1024;
    let mut buffer = vec![0u8; property_size as usize];
    let mut property_type = 0;

    if SetupDiGetDeviceRegistryPropertyW(
        dev_set,
        dev_inf,
        property,
        Some(&mut property_type),
        Some(&mut buffer),
        Some(&mut property_size),
    )
    .is_ok()
    {
        buffer.truncate(property_size as usize);
        let utf16: Vec<u16> = buffer
            .chunks_exact(2)
            .map(|c| u16::from_ne_bytes([c[0], c[1]]))
            .collect();
        return utf16_to_utf8(&utf16);
    }
    return None;
}

unsafe fn get_serial_info(
    name: String,
    dev_set: HDEVINFO,
    dev_inf: &SP_DEVINFO_DATA,
) -> SerialInfo {
    return SerialInfo {
        name,
        valid: true,
        vendor: get_serial_property(dev_set, dev_inf, SPDRP_MFG),
        product: get_serial_property(dev_set, dev_inf, SPDRP_DEVICEDESC),
        driver: None,
        usb_info: get_usb_info(dev_set, dev_inf),
    };
}

/// enumerate all avaliable serial port
pub fn get_serial_list() -> Vec<SerialInfo> {
    let mut serial_list = Vec::new();
    let guids = get_device_class_guids_form_serial();
    for guid in guids.unwrap() {
        unsafe {
            let dev_set = match SetupDiGetClassDevsA(Some(&guid), None, None, DIGCF_PRESENT) {
                Ok(dev_set) => dev_set,
                Err(_) => continue,
            };
            let mut dev_cnt = 0;
            loop {
                let mut dev_info = SP_DEVINFO_DATA {
                    cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
                    ClassGuid: GUID::zeroed(),
                    DevInst: 0,
                    Reserved: 0,
                };

                let result = SetupDiEnumDeviceInfo(dev_set, dev_cnt, &mut dev_info);
                if result.is_err() {
                    break;
                }
                dev_cnt += 1;

                let port_name = get_port_name_from_dev_info(dev_set, &dev_info);
                match port_name {
                    Some(port_name) => {
                        if port_name.starts_with("COM") {
                            let serial_info = get_serial_info(port_name, dev_set, &dev_info);
                            serial_list.push(serial_info);
                        }
                    }
                    None => {}
                }
            }
            SetupDiDestroyDeviceInfoList(dev_set).ok();
        }
    }
    return serial_list;
}
