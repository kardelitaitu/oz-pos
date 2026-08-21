use super::*;

#[test]
fn new_stores_fields() {
    let info = DeviceInfo::new("Epson", "TM-m30", "SN001");
    let printer = BtReceiptPrinter::new("COM7", 9600, info.clone());
    assert_eq!(printer.port_name, "COM7");
    assert_eq!(printer.baud_rate, 9600);
    assert!(!printer.partial_cut);
}

#[test]
fn device_info_returns_identity() {
    let info = DeviceInfo::new("Star", "SP700", "SN002");
    let printer = BtReceiptPrinter::new("/dev/rfcomm0", 115200, info.clone());
    let returned = printer.device_info();
    assert_eq!(returned.vendor, "Star");
    assert_eq!(returned.model, "SP700");
    assert_eq!(returned.serial, "SN002");
}

#[test]
fn with_partial_cut_enables() {
    let info = DeviceInfo::new("Test", "Printer", "SN");
    let printer = BtReceiptPrinter::new("COM1", 9600, info).with_partial_cut(true);
    assert!(printer.partial_cut);
}

#[test]
fn with_partial_cut_disables() {
    let info = DeviceInfo::new("Test", "Printer", "SN");
    let printer = BtReceiptPrinter::new("COM1", 9600, info).with_partial_cut(false);
    assert!(!printer.partial_cut);
}

#[test]
fn default_partial_cut_is_false() {
    let info = DeviceInfo::new("Test", "Printer", "SN");
    let printer = BtReceiptPrinter::new("COM1", 9600, info);
    assert!(!printer.partial_cut);
}
