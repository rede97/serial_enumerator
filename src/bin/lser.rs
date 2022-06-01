use serial_enumerator::get_serial_list;

fn main() {
    let mut serials_info = get_serial_list();
    serials_info.sort_by(|a, b| a.name.cmp(&b.name));
    for serial_info in serials_info {
        println!("{:?}", serial_info);
    }
}
