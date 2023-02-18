use crate::{SerialInfo, UsbInfo};

extern crate IOKit_sys;
extern crate CoreFoundation_sys as cf;
extern crate libc;
extern crate mach;

use std::mem;

use std::ffi::{CString,CStr};

use libc::{c_char,c_void};

use mach::port::{mach_port_t,MACH_PORT_NULL};
use mach::kern_return::KERN_SUCCESS;

use IOKit_sys::*;
use cf::*;

use defer_lite::defer;

/// Returns a specific property of the given device as a string.
fn get_string_property(device_type: io_registry_entry_t, property: &str) -> Option<String> {
    unsafe {
        let prop_str = CString::new(property).unwrap();
        let key = CFStringCreateWithCString(
            kCFAllocatorDefault,
            prop_str.as_ptr(),
            kCFStringEncodingUTF8,
        );
        if key.is_null() {
            panic!("failed to allocate key string");
        }
        defer! {CFRelease(key as *mut c_void)}
        let container = IORegistryEntryCreateCFProperty(device_type, key, kCFAllocatorDefault, 0);
        if container.is_null() {
            return None;
        }
        defer! {CFRelease(container)}

        let mut buf = Vec::with_capacity(256);
        let result = CFStringGetCString(
            container as CFStringRef,
            buf.as_mut_ptr(),
            buf.capacity() as i64,
            kCFStringEncodingUTF8,
        );
        let opt_str = if result != 0 {
            CStr::from_ptr(buf.as_ptr()).to_str().ok().map(String::from)
        } else {
            None
        };

        opt_str
    }
}

/// Returns a specific property of the given device as an integer.
#[allow(non_upper_case_globals)]
fn get_int_property(
    device_type: io_registry_entry_t,
    property: &str,
    cf_number_type: CFNumberType,
) -> Option<u32> {
    unsafe {
        let prop_str = CString::new(property).unwrap();
        let key = CFStringCreateWithCString(
            kCFAllocatorDefault,
            prop_str.as_ptr(),
            kCFStringEncodingUTF8,
        );
        if key.is_null() {
            panic!("failed to allocate key string");
        }
        defer! {CFRelease(key as *mut c_void)}
        let container = IORegistryEntryCreateCFProperty(device_type, key, kCFAllocatorDefault, 0);
        if container.is_null() {
            return None;
        }
        defer! {CFRelease(container)}

        let num = match cf_number_type {
            kCFNumberSInt8Type => {
                let mut num: u8 = 0;
                let num_ptr: *mut c_void = &mut num as *mut _ as *mut c_void;
                CFNumberGetValue(container as CFNumberRef, cf_number_type, num_ptr);
                Some(num as u32)
            }
            kCFNumberSInt16Type => {
                let mut num: u16 = 0;
                let num_ptr: *mut c_void = &mut num as *mut _ as *mut c_void;
                CFNumberGetValue(container as CFNumberRef, cf_number_type, num_ptr);
                Some(num as u32)
            }
            kCFNumberSInt32Type => {
                let mut num: u32 = 0;
                let num_ptr: *mut c_void = &mut num as *mut _ as *mut c_void;
                CFNumberGetValue(container as CFNumberRef, cf_number_type, num_ptr);
                Some(num)
            }
            _ => None,
        };

        num
    }
}

fn get_parent_device_by_type(
    device: io_object_t,
    parent_type: *const c_char,
) -> Option<io_registry_entry_t> {
    let parent_type = unsafe { CStr::from_ptr(parent_type) };
    let mut device = device;
    loop {
        let mut class_name = mem::MaybeUninit::<[c_char; 128]>::uninit();
        unsafe { IOObjectGetClass(device, class_name.as_mut_ptr() as *mut c_char) };
        let class_name = unsafe { class_name.assume_init() };
        let name = unsafe { CStr::from_ptr(&class_name[0]) };
        if name == parent_type {
            return Some(device);
        }
        let mut parent = mem::MaybeUninit::uninit();
        if unsafe {
            IORegistryEntryGetParentEntry(device, kIOServiceClass(), parent.as_mut_ptr())
                != KERN_SUCCESS
        } {
            return None;
        }
        device = unsafe { parent.assume_init() };
    }
}

fn get_serial_info(modem_service: io_iterator_t, name: &str) -> SerialInfo {
    let usb_device_class_name = b"IOUSBHostDevice\0".as_ptr() as *const c_char;
    let legacy_usb_device_class_name = kIOUSBDeviceClassName();

    let maybe_usb_device = get_parent_device_by_type(modem_service, usb_device_class_name)
        .or_else(|| get_parent_device_by_type(modem_service, legacy_usb_device_class_name));
    if let Some(usb_device) = maybe_usb_device {
        let vid = get_int_property(usb_device, "idVendor", kCFNumberSInt16Type).unwrap_or_default();
        let pid = get_int_property(usb_device, "idProduct", kCFNumberSInt16Type).unwrap_or_default();
        let vendor = get_string_property(usb_device, "USB Vendor Name");
        let product = get_string_property(usb_device, "USB Product Name");

        return SerialInfo {
            name: name.to_string(),
            vendor,
            product,
            driver: None,
            usb_info: Some(UsbInfo { vid: format!("{:x}", vid), pid: format!("{:x}", pid) })
        }
    }
    return SerialInfo {
        name: name.to_string(),
        vendor: None,
        product: None,
        driver: None,
        usb_info: None
    };
}

fn get_serial_info_callout_dialin(modem_service: io_iterator_t, props: CFMutableDictionaryRef) -> Result<Vec<SerialInfo>, String> {
    unsafe {
        let mut vec = vec![];

        for key in ["IOCalloutDevice", "IODialinDevice"].iter() {
            let key = CString::new(*key).unwrap();
            let key_cfstring = CFStringCreateWithCString(
                kCFAllocatorDefault,
                key.as_ptr(),
                kCFStringEncodingUTF8,
            );
            if key_cfstring.is_null() {
                panic!("could not allocate CFString");
            }
            defer! {CFRelease(key_cfstring as *mut c_void)}
            let value = CFDictionaryGetValue(props, key_cfstring as *const c_void);

            let type_id = CFGetTypeID(value);
            if type_id == CFStringGetTypeID() {
                let mut buf = Vec::with_capacity(256);

                CFStringGetCString(
                    value as CFStringRef,
                    buf.as_mut_ptr(),
                    256,
                    kCFStringEncodingUTF8,
                );
                let path = CStr::from_ptr(buf.as_ptr()).to_string_lossy();

                let si = get_serial_info(modem_service, &path);

                vec.push(si);
            } else {
                return Err("type id did not match".to_string()); // TODO: return a sensible error
            }
        }

        return Ok(vec);
    }
}

fn iokit_list() -> Result<Vec<SerialInfo>, String> {
    unsafe {
        let mut master_port: mach_port_t = MACH_PORT_NULL;

        let classes_to_match = IOServiceMatching(kIOSerialBSDServiceValue());
        if classes_to_match.is_null() {
            panic!("IOServiceMatching returned a NULL dictionary.");
        }

        // build key
        let key = CFStringCreateWithCString(kCFAllocatorDefault, kIOSerialBSDTypeKey(), kCFStringEncodingUTF8);
        if key.is_null() {
            panic!("failed to allocate key string");
        }
        defer! {CFRelease(key as *mut c_void)}

        // build value
        let val = CFStringCreateWithCString(kCFAllocatorDefault, kIOSerialBSDAllTypes(), kCFStringEncodingUTF8);
        if val.is_null() {
            panic!("failed to allocate value string");
        }
        defer! {CFRelease(val as *mut c_void)}

        // set value in dictionary
        CFDictionarySetValue(classes_to_match, key as CFTypeRef, val as CFTypeRef);

        let mut kern_result = IOMasterPort(MACH_PORT_NULL, &mut master_port);
        if kern_result != KERN_SUCCESS {
            panic!("ERROR: {}", kern_result);
        }

        let mut matching_services = mem::MaybeUninit::<io_iterator_t>::uninit();

        kern_result = IOServiceGetMatchingServices(kIOMasterPortDefault, classes_to_match, matching_services.as_mut_ptr());
        if kern_result != KERN_SUCCESS {
            panic!("ERROR: {}", kern_result);
        }

        let mut infos = vec![];
        loop {
            let modem_service = IOIteratorNext(matching_services.assume_init());
            if modem_service == MACH_PORT_NULL {
                break;
            }
            defer! {IOObjectRelease(modem_service);}

            let mut props = mem::MaybeUninit::<CFMutableDictionaryRef>::uninit();
            let result = IORegistryEntryCreateCFProperties(modem_service, props.as_mut_ptr(), kCFAllocatorDefault, 0);
            if result == KERN_SUCCESS {
                defer! {CFRelease(props.assume_init() as *mut c_void)}
                let mut r = get_serial_info_callout_dialin(modem_service, props.assume_init())?;
                infos.append(&mut r)
            } else {
                return Err("could not get properties of modem service".to_string());
            }
        }

        return Ok(infos);
    }
}

pub fn get_serial_list() -> Vec<SerialInfo> {
    match iokit_list() {
        Ok(v) => return v,
        Err(s) => panic!("{}", s)
    }
}