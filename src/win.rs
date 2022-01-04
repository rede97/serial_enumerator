use crate::{SerialInfo, UsbInfo};
use core::ffi::c_void;
use std::mem::size_of;
use windows::core::GUID;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiClassGuidsFromNameA, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsA, SetupDiGetDeviceInstanceIdA, SetupDiGetDeviceRegistryPropertyW,
    SetupDiOpenDevRegKey, DICS_FLAG_GLOBAL, DIGCF_PRESENT, DIREG_DEV, SPDRP_DEVICEDESC, SPDRP_MFG,
    SP_DEVINFO_DATA,
};
use windows::Win32::Foundation::PSTR;
use windows::Win32::System::Registry::{RegCloseKey, RegQueryValueExA, KEY_READ};

mod device_id_parser {
    use nom::branch::alt;
    use nom::bytes::complete::{tag, take_while_m_n};
    use nom::sequence::{delimited, preceded, separated_pair};
    use nom::{AsChar, IResult};

    fn usb_prefix_parser(s: &str) -> IResult<&str, &str> {
        return alt((tag("USB"), tag("FTDIBUS")))(s);
    }

    fn usbid_parser(s: &str) -> IResult<&str, &str> {
        return take_while_m_n(4, 4, |c: char| c.is_hex_digit())(s);
    }

    fn vid_pid_parser(s: &str) -> IResult<&str, (&str, &str)> {
        return delimited(
            tag("\\"),
            separated_pair(
                preceded(tag("VID_"), usbid_parser),
                tag("&"),
                preceded(tag("PID_"), usbid_parser),
            ),
            tag("\\"),
        )(s);
    }

    fn device_id_parser(s: &str) -> IResult<&str, (&str, &str)> {
        return preceded(usb_prefix_parser, vid_pid_parser)(s);
    }

    /// parse line of /proc/tty/drivers
    pub fn parse_device_id(device_id: &str) -> Option<(String, String)> {
        match device_id_parser(device_id) {
            Ok((_, (vid, pid))) => {
                return Some((vid.into(), pid.into()));
            }
            Err(_) => return None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn test_drivers_line_parse() {
            let result = parse_device_id(r"USB\VID_1A86&PID_7523\7&139F9FFA&0&2").unwrap();
            assert_eq!(result, ("1A86".into(), "7523".into()));
        }
    }
}

fn get_device_class_guids_form_serial() -> Option<Vec<GUID>> {
    const DEVICE_NAME: &str = "Ports";
    let mut required_size: u32 = 0;
    unsafe {
        SetupDiClassGuidsFromNameA(DEVICE_NAME, std::ptr::null_mut(), 0, &mut required_size);
    }
    let mut guids = Vec::with_capacity(required_size as usize);
    let status = unsafe {
        let status = SetupDiClassGuidsFromNameA(
            DEVICE_NAME,
            guids.as_mut_ptr(),
            required_size,
            &mut required_size,
        );
        guids.set_len(required_size as usize);
        status
    };

    if status.as_bool() {
        return Some(guids);
    } else {
        return None;
    }
}

unsafe fn get_port_name_from_dev_info(
    dev_set: *const c_void,
    dev_inf: &SP_DEVINFO_DATA,
) -> Option<String> {
    let hkey = SetupDiOpenDevRegKey(dev_set, dev_inf, DICS_FLAG_GLOBAL, 0, DIREG_DEV, KEY_READ);
    if hkey == 0 {
        return None;
    }
    let mut name_size: u32 = 1024;
    let mut ltype = 0;
    let mut buffer = Vec::with_capacity(name_size as usize);
    let result = RegQueryValueExA(
        hkey,
        "PortName",
        std::ptr::null_mut(),
        &mut ltype,
        buffer.as_mut_ptr(),
        &mut name_size,
    );
    RegCloseKey(hkey);
    if result == 0 {
        buffer.set_len(name_size as usize - 1);
        let port_name = String::from_utf8(buffer).unwrap();
        return Some(port_name);
    }
    return None;
}

unsafe fn get_usb_info(dev_set: *const c_void, dev_inf: &SP_DEVINFO_DATA) -> Option<UsbInfo> {
    let mut id_size: u32 = 1024;
    let mut buffer = Vec::with_capacity(id_size as usize);
    if SetupDiGetDeviceInstanceIdA(
        dev_set,
        dev_inf,
        PSTR(buffer.as_mut_ptr()),
        id_size,
        &mut id_size,
    )
    .as_bool()
    {
        buffer.set_len(id_size as usize - 1);
        let device_id = String::from_utf8(buffer).unwrap();
        return device_id_parser::parse_device_id(device_id.as_str())
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
    dev_set: *const c_void,
    dev_inf: &SP_DEVINFO_DATA,
    mut property: u32,
) -> Option<String> {
    let mut property_size: u32 = 1024;
    let mut buffer: Vec<u16> = Vec::with_capacity(property_size as usize);

    if SetupDiGetDeviceRegistryPropertyW(
        dev_set,
        dev_inf,
        property,
        &mut property,
        buffer.as_mut_ptr() as *mut u8,
        property_size,
        &mut property_size,
    )
    .as_bool()
    {
        buffer.set_len(property_size as usize);
        return utf16_to_utf8(&buffer);
    }
    return None;
}

unsafe fn get_serial_info(
    name: String,
    dev_set: *const c_void,
    dev_inf: &SP_DEVINFO_DATA,
) -> SerialInfo {
    return SerialInfo {
        name,
        vendor: get_serial_property(dev_set, dev_inf, SPDRP_MFG),
        product: get_serial_property(dev_set, dev_inf, SPDRP_DEVICEDESC),
        driver: None,
        usb_info: get_usb_info(dev_set, dev_inf),
    };
}

pub fn get_serial_list() -> Vec<SerialInfo> {
    let mut serial_list = Vec::new();
    let guids = get_device_class_guids_form_serial();
    for guid in guids.unwrap() {
        unsafe {
            let dev_set = SetupDiGetClassDevsA(&guid, None, None, DIGCF_PRESENT);
            if dev_set > std::ptr::null_mut() {
                let mut dev_cnt = 0;
                loop {
                    let mut dev_info = SP_DEVINFO_DATA {
                        cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
                        ClassGuid: GUID::zeroed(),
                        DevInst: 0,
                        Reserved: 0,
                    };

                    let result = SetupDiEnumDeviceInfo(dev_set, dev_cnt, &mut dev_info);
                    if !result.as_bool() {
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
                SetupDiDestroyDeviceInfoList(dev_set);
            }
        }
    }
    return serial_list;
}
