use std::time::Duration;
use std::thread;
use std::ops::{Deref, DerefMut};
use btleplug::api::{ScanFilter, Peripheral as _, Central as _};
use btleplug::platform::Peripheral;
use crate::CENTRAL;

pub struct ScanResult {
    pub result: Vec<Peripheral>,
}

impl ScanResult {
    pub async fn get_by_name(&self, name: &str) -> Option<Peripheral> {
        for device in self.iter() {
            let props = device.properties().await.unwrap().unwrap();
            if props.local_name == Some(name.to_string()) {
                return Some(device.clone());
            }
        }

        return None;
    }

    pub async fn get_by_addr(&self, id: &str) -> Option<Peripheral> {
        for device in self.iter() {
            let props: btleplug::api::PeripheralProperties = device.properties().await.unwrap().unwrap();
            if props.address.to_string().to_lowercase() == id.to_lowercase() {
                return Some(device.clone());
            }
        }

        return None;
    }
}

impl Deref for ScanResult {
    type Target = Vec<Peripheral>;
    fn deref(&self) -> &Vec<Peripheral> { &self.result }
}

impl DerefMut for ScanResult {
    fn deref_mut(&mut self) -> &mut Vec<Peripheral> { &mut self.result }
}

/// Scan for BLE devices
pub async fn scan_bluetooth(time: u8) -> ScanResult {
    CENTRAL.get().await
        .start_scan(ScanFilter::default())
        .await
        .expect("Couldn't scan for BLE devices...");

    thread::sleep(Duration::from_secs(time as u64)); // Wait until the scan is done

    let result = CENTRAL.get().await
        .peripherals()
        .await
        .expect("Couldn't retrieve peripherals from BLE adapter...");

    ScanResult {
        result
    }
}