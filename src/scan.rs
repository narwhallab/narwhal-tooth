use std::time::Duration;
use btleplug::{api::{ScanFilter, Central as _, Peripheral as _}, platform::{Peripheral, Adapter}};
use crate::CENTRAL;

#[derive(Clone)]
pub struct BluetoothDevice {
    addr: String,
    local_name: Option<String>
}

impl BluetoothDevice {
    pub fn get_local_name(&self) -> Option<String> {
        self.local_name.clone()
    }

    pub fn get_addr(&self) -> String {
        self.addr.clone()
    }
}

pub struct ScanResult {
    pub(crate) scanned_devices: Vec<BluetoothDevice>,
}

impl ScanResult {
    pub async fn search_by_name(&self, name: String) -> Option<BluetoothDevice> {
        for device in self.scanned_devices.iter() {
            if let Some(local_name) = &device.local_name {
                if local_name == &name {
                    return Some(device.clone());
                }
            }
        }

        return None;
    }

    pub async fn search_by_addr(&self, id: String) -> Option<BluetoothDevice> {
        for device in self.scanned_devices.iter() {
            if device.addr.to_lowercase() == id.to_lowercase() {
                return Some(device.clone());
            }
        }

        return None;
    }
}

pub async fn scan_bluetooth(duration: Duration) -> ScanResult {
    let adapter = CENTRAL.get().await;
    
    adapter.start_scan(ScanFilter::default()).await.expect("Couldn't scan for BLE devices...");

    tokio::time::sleep(duration).await; // Wait until the scan is done

    let scanned_peripherals = load_peripherals(adapter).await;

    let mut scanned_devices = vec![];

    for peripheral in scanned_peripherals.iter() {
        scanned_devices.push(peripheral_to_bluetooth_device(peripheral).await);
    }

    return ScanResult {
        scanned_devices
    }
}

pub(crate) async fn load_peripherals(adapter: &Adapter) -> Vec<Peripheral> {
    let peripherals = adapter
        .peripherals()
        .await
        .expect("Couldn't retrieve peripherals from BLE adapter...");

    return peripherals
}

pub(crate) async fn peripheral_to_bluetooth_device(peripheral: &Peripheral) -> BluetoothDevice {
    let props = peripheral.properties().await.unwrap().unwrap();
    let local_name = props.local_name;
    let addr = props.address.to_string();

    return BluetoothDevice { 
        addr, 
        local_name
    }
}

pub(crate) async fn bluetooth_device_to_peripheral(device: BluetoothDevice, peripherals: Vec<Peripheral>) -> Option<Peripheral> {
    for peripheral in peripherals.iter() {
        let props = peripheral.properties().await.unwrap().unwrap();
        if device.addr == props.address.to_string() {
            return Some(peripheral.clone())
        }
    }

    return None
}