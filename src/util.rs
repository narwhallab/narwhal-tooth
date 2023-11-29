use std::{error::Error, str::FromStr as _};

use btleplug::{platform::{Adapter, Peripheral, Manager}, api::{Peripheral as _, CharPropFlags, BDAddr, Central as _, Manager as _}};
use log::info;

use crate::{bluetooth::BluetoothConnection, CENTRAL};

pub async fn fetch_adapter() -> Adapter {
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

pub async fn connect_peripheral(peripheral: &Peripheral) -> Result<BluetoothConnection, Box<dyn Error>>  {
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
                return Ok(BluetoothConnection { peripheral: peripheral.clone(), target_characteristic: characteristic });
            }
        }
    }

    return Err("Could not find a valid characteristic for read/write".into())
}

pub async fn peripheral_by_addr(addr: &str) -> Result<Peripheral, Box<dyn Error>> {
    let adapter = CENTRAL.get().await;

    // todo: add scan process

    let peripherals = adapter.peripherals().await?;

    let peripheral = peripherals
        .iter()
        .find(|&p| p.address().eq(&BDAddr::from_str(addr).unwrap()))
        .expect("Could not find a peripheral with the matching address")
        .to_owned();

    return Ok(peripheral)
}

pub async fn connect_by_address(address: String) -> BluetoothConnection {
    let peripheral = peripheral_by_addr(&address).await.expect("Failed to find peripheral");
    return connect_peripheral(&peripheral).await.expect("Error")
}