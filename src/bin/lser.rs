use serial_enumerator::get_serial_list;

fn main() {
    let serials = get_serial_list();
    for serial in serials {
        println!("{:?}", serial);
    }
}
