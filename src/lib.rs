pub mod bluetooth;

use std::time::Duration;
use std::thread;
use std::ops::{Deref, DerefMut};
use async_once::AsyncOnce;
use btleplug::api::{Central, ScanFilter, Peripheral as _};
use btleplug::platform::{Adapter, Peripheral};
use simplelog::{TermLogger, Config};
use lazy_static::lazy_static;
use bluetooth::{get_adapter, connect_peripheral, BluetoothConnection, connect_peripheral_by_address};

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

pub async fn scan_bluetooth(time: u8) -> ScanResult {
    CENTRAL.get().await
        .start_scan(ScanFilter::default())
        .await
        .expect("Can't scan BLE adapter for connected devices...");

    thread::sleep(Duration::from_secs(time as u64)); // Wait until the scan is done

    let peripherals = CENTRAL.get().await
        .peripherals()
        .await
        .expect("Can't get peripherals from BLE adapter...");

    ScanResult {
        result: peripherals
    }
}

pub async fn connect_by_address(address: String) -> BluetoothConnection {
    connect_peripheral_by_address(&address).await.expect("Error")
}

pub async fn connect_bluetooth(peripheral: &Peripheral) -> BluetoothConnection {
    connect_peripheral(peripheral).await.expect("Error")
}

#[cfg(test)]
mod test {
    use crate::{scan_bluetooth, connect_bluetooth};

    #[tokio::test]
    async fn test_ble() {
        let result = scan_bluetooth(3).await;
        let hmsoft = result.get_by_name("HMSoft").await.expect("Could not find HMSoft device");
        let connection = connect_bluetooth(&hmsoft).await;
        connection.write("off".as_bytes()).await.unwrap();
        connection.disconnect().await;
    }
}
