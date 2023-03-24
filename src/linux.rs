use crate::{SerialInfo, UsbInfo};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::vec::Vec;

mod tty_drivers_parser {
    use nom::bytes::complete::{tag, take_till1};
    use nom::character::complete::space1;
    use nom::sequence::{preceded, separated_pair};
    use nom::IResult;

    fn class_prefix_parser(s: &str) -> IResult<&str, &str> {
        return take_till1(|c: char| c.is_ascii_whitespace())(s);
    }

    fn prefix_parser(s: &str) -> IResult<&str, &str> {
        return preceded(tag("/dev/"), class_prefix_parser)(s);
    }

    fn drivers_line_parser(s: &str) -> IResult<&str, (&str, &str)> {
        return separated_pair(class_prefix_parser, space1, prefix_parser)(s);
    }

    /// parse line of /proc/tty/drivers
    pub fn parse_line(line: &str) -> Option<(String, String)> {
        if line.ends_with("serial") {
            let (_, (class, prefix)) = drivers_line_parser(line).expect(line);
            return Some((class.into(), prefix.into()));
        }
        return None;
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn test_drivers_line_parse() {
            let result =
                parse_line("serial               /dev/ttyS       4 64-111 serial").unwrap();
            assert_eq!(result, ("serial".into(), "ttyS".into()));
        }
    }
}

fn get_serial_prefix() -> HashMap<String, String> {
    let mut serial_prefix = HashMap::new();
    const TTY_DRIVERS: &str = "/proc/tty/drivers";

    match fs::read(TTY_DRIVERS) {
        Ok(result) => {
            let tty_drivers_info = String::from_utf8(result).unwrap();
            for line in tty_drivers_info.lines() {
                match tty_drivers_parser::parse_line(line) {
                    Some((class, prefix)) => {
                        serial_prefix.insert(prefix, class);
                    }
                    None => {}
                }
            }
        }
        Err(_e) => {
            serial_prefix.insert("ttyS".into(), "serial".into());
            serial_prefix.insert("ttyUSB".into(), "usbserial".into());
            serial_prefix.insert("ttyPS".into(), "other".into());
            serial_prefix.insert("ttyACM".into(), "other".into());
            serial_prefix.insert("ttyAMA".into(), "other".into());
            serial_prefix.insert("ttymxc".into(), "other".into());
            serial_prefix.insert("ttyGS".into(), "other".into());
        }
    }
    return serial_prefix;
}

fn read_line(path: &PathBuf) -> Option<String> {
    match fs::read(path) {
        Ok(raw) => {
            let text = String::from_utf8(raw).expect(path.to_str().unwrap());
            return text.lines().next().and_then(|l| Some(l.replace("\0", ";")));
        }
        Err(_) => {
            return None;
        }
    }
}

fn get_file_name(path: &PathBuf) -> Option<String> {
    return path
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(|s| Some(String::from(s)));
}

fn get_file_real_name(device_path: &PathBuf, name: &str) -> Option<String> {
    let mut file_path = device_path.clone();
    file_path.push(name);
    let real_file_path = fs::canonicalize(&file_path).expect(file_path.to_str().unwrap());
    return get_file_name(&real_file_path);
}

fn probe_usb_serial(mut real_dev_path: PathBuf, serial_info: &mut SerialInfo) -> bool {
    let mut interface_num = None;
    for _ in 0..3 {
        // read interface
        if interface_num.is_none() {
            real_dev_path.push("bInterfaceNumber");
            interface_num = read_line(&real_dev_path);
            real_dev_path.pop();
        }

        // read vendor
        real_dev_path.push("manufacturer");
        serial_info.vendor = read_line(&real_dev_path);
        real_dev_path.pop();

        // read product
        real_dev_path.push("product");
        serial_info.product = read_line(&real_dev_path).and_then(|mut product| {
            if let Some(iface_num) = &interface_num {
                // For example: FT2232 with dual port serial
                product.push(':');
                product.push_str(&iface_num);
                return Some(product);
            }
            Some(product)
        });
        real_dev_path.pop();
        // read vid and pid
        real_dev_path.push("idVendor");
        let vid = read_line(&real_dev_path);
        real_dev_path.pop();
        real_dev_path.push("idProduct");
        let pid = read_line(&real_dev_path);
        real_dev_path.pop();
        if let (Some(vid), Some(pid)) = (vid, pid) {
            serial_info.usb_info = Some(UsbInfo { vid, pid });
        }

        if serial_info.vendor.is_none()
            && serial_info.product.is_none()
            && serial_info.usb_info.is_none()
        {
            real_dev_path.push("../");
        } else {
            return true;
        }
    }
    return false;
}

fn probe_acm_serial(mut real_dev_path: PathBuf, serial_info: &mut SerialInfo) -> bool {
    real_dev_path.push("subsystem");
    let dev_subsystem = fs::canonicalize(&real_dev_path).expect(real_dev_path.to_str().unwrap());
    real_dev_path.pop();

    if dev_subsystem.ends_with("usb") {
        return probe_usb_serial(real_dev_path, serial_info);
    }
    return true;
}

fn probe_builtin_serial(mut real_dev_path: PathBuf, serial_info: &mut SerialInfo) -> bool {
    // declared in device tree
    real_dev_path.push("of_node");
    let is_exist_ofnode = real_dev_path.exists();
    real_dev_path.pop();

    // pnp serial
    real_dev_path.push("id");
    let is_exist_id = real_dev_path.exists();
    real_dev_path.pop();

    if is_exist_ofnode || is_exist_id {
        serial_info.vendor = get_file_real_name(&real_dev_path, "subsystem");

        if is_exist_ofnode {
            // compatible property of device tree
            real_dev_path.push("of_node/compatible");
        } else {
            // pnp id
            real_dev_path.push("id");
        }
        serial_info.product = read_line(&real_dev_path);
        return true;
    }
    return false;
}

fn probe_serial_by_prefix(
    serial_list: &mut Vec<SerialInfo>,
    serial_prefix: &HashMap<String, String>,
) {
    const TTY_DEVICE_PATH: &str = "/sys/class/tty";
    for _entry in fs::read_dir(TTY_DEVICE_PATH).unwrap() {
        if let Ok(entry) = _entry {
            let _file_name = entry.file_name();
            let file_name = _file_name.to_str().expect(format!("{:?}", entry).as_str());
            for (prefix, driver_class) in serial_prefix {
                if file_name.starts_with(prefix) {
                    let mut device_path = entry.path();
                    device_path.push("device");
                    let real_dev_path =
                        fs::canonicalize(&device_path).expect(device_path.to_str().unwrap());
                    let mut serial_info = SerialInfo {
                        name: format!("/dev/{}", file_name),
                        vendor: None,
                        product: None,
                        driver: get_file_real_name(&real_dev_path, "driver"),
                        usb_info: None,
                    };
                    if real_dev_path.exists() {
                        let is_valid_serial = if file_name.starts_with("ttyACM") {
                            probe_acm_serial(real_dev_path, &mut serial_info)
                        } else {
                            match driver_class.as_str() {
                                "usbserial" => probe_usb_serial(real_dev_path, &mut serial_info),
                                _ => probe_builtin_serial(real_dev_path, &mut serial_info),
                            }
                        };
                        if is_valid_serial {
                            serial_list.push(serial_info);
                        }
                    }
                    break;
                }
            }
        }
    }
}

/// enumerate all avaliable serial port
pub fn get_serial_list() -> Vec<SerialInfo> {
    let mut serial_list = Vec::new();
    let serial_prefix = get_serial_prefix();
    probe_serial_by_prefix(&mut serial_list, &serial_prefix);
    return serial_list;
}
