use std::error::Error;

use btleplug::{platform::{Adapter, Manager}, api::{Peripheral as _, CharPropFlags, Manager as _}};
use log::info;

use crate::{bluetooth::BluetoothConnection, CENTRAL, scan::{BluetoothDevice, load_peripherals, bluetooth_device_to_peripheral}};

pub(crate) async fn fetch_adapter() -> Adapter {
    let manager = Manager::new().await.expect("Could not fetch manager");

    let adapters = manager
        .adapters()
        .await
        .expect("Unable to fetch adapter list.");

    return adapters
        .into_iter()
        .nth(0)
        .expect("No adapters are available now...")
}

pub async fn connect_device(device: BluetoothDevice) -> Result<BluetoothConnection, Box<dyn Error>>  {
    let adapter = CENTRAL.get().await;
    let peripherals = load_peripherals(adapter).await;

    let peripheral = bluetooth_device_to_peripheral(device.clone(), peripherals).await.expect("Invalid Bluetooth Device");

    let properties = peripheral.properties().await?;
    let is_connected = peripheral.is_connected().await?;
    let local_name = properties
        .unwrap()
        .local_name
        .unwrap_or(String::from("<no name>"));

    // attempt peripheral connection
    if !is_connected {
        info!("Connecting to peripheral {}...", &local_name);
        peripheral.connect().await?;
    }

    // failed to connect
    if !peripheral.is_connected().await? {
        return Err("Could not connect to peripheral".into());
    }

    peripheral.discover_services().await?;

    for service in peripheral.services() {
        for characteristic in service.characteristics {
            if characteristic.properties.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE) {
                let connection = BluetoothConnection { 
                    peripheral: peripheral.clone(), 
                    target_characteristic: characteristic,
                    device: device.clone()
                };

                connection.initialize().await;

                return Ok(connection);
            }
        }
    }

    return Err("Could not find a valid characteristic for read/write".into())
}