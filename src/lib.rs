pub mod bluetooth;

use std::time::Duration;
use std::thread;
use std::ops::{Deref, DerefMut};
use async_once::AsyncOnce;
use btleplug::api::{Central, ScanFilter, Peripheral as _};
use btleplug::platform::{Adapter, Peripheral};
use simplelog::{TermLogger, Config};
use lazy_static::lazy_static;
use bluetooth::{get_adapter, connect_peripheral, BluetoothConnection, peripheral_by_addr};

lazy_static! {
    pub static ref CENTRAL: AsyncOnce<Adapter> = AsyncOnce::new(async {
        get_adapter().await
    });
}

pub fn init_logger() {
    TermLogger::init(log::LevelFilter::Info, Config::default(), simplelog::TerminalMode::Stdout, simplelog::ColorChoice::Auto).unwrap();
}

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

pub async fn connect_by_address(address: String) -> BluetoothConnection {
    let peripheral = peripheral_by_addr(&address).await.expect("Failed to find peripheral");
    connect_peripheral(&peripheral).await.expect("Error")
}

#[cfg(test)]
mod test {
    use crate::{scan_bluetooth, bluetooth::connect_peripheral};

    #[tokio::test]
    async fn test_ble() {
        let result = scan_bluetooth(5).await;
        let hmsoft = result.get_by_name("HMSoft").await.expect("Could not find HMSoft device");
        let connection = connect_peripheral(&hmsoft).await.unwrap();

        connection.write("on".as_bytes()).await.unwrap();
        connection.disconnect().await.unwrap();
        println!("{}", connection.peripheral_connected().await)
    }
}
