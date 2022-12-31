use std::{error::Error, str::FromStr};
use btleplug::{platform::{Adapter, Manager, Peripheral}, api::{Manager as _, Central as _, CharPropFlags, WriteType, Peripheral as _, BDAddr, PeripheralProperties}};
use futures::StreamExt;
use log::info;

use crate::CENTRAL;

pub async fn get_adapter() -> Adapter {
    let manager = Manager::new().await.expect("Could not fetch manager");

    manager
        .adapters()
        .await
        .expect("Unable to fetch adapter list.")
        .into_iter()
        .nth(0)
        .expect("No adapters are available now...")
}

pub async fn peripheral_by_addr(addr: &BDAddr) -> Result<Peripheral, Box<dyn Error>> {
    let peripheral = CENTRAL.get()
        .await
        .peripherals()
        .await?
        .iter()
        .find(|p| p.address().eq(addr))
        .expect("Error")
        .to_owned();

    Ok(peripheral)
}

pub async fn connect_peripheral(peripheral: &Peripheral) -> Result<BluetoothConnection, Box<dyn Error>>  {
    let properties = peripheral.properties().await?;
    let is_connected = peripheral.is_connected().await?;
    let local_name = properties
        .unwrap()
        .local_name
        .unwrap_or(String::from("<no name>"));

    if !is_connected {
        info!("Connecting to peripheral {}...", &local_name);
        peripheral.connect().await?;
    }

    let is_connected = peripheral.is_connected().await?;
    let message = if is_connected { "succeeded" } else { "failed" };

    info!("Connection to peripherial: {} has {}", &local_name, message);
    Ok(BluetoothConnection { peripheral: peripheral.clone() })
}

pub async fn connect_peripheral_by_address(address: &str) -> Result<BluetoothConnection, Box<dyn Error>> {
    let peripheral = peripheral_by_addr(&BDAddr::from_str(address).unwrap()).await?;

    connect_peripheral(&peripheral).await
}

#[derive(Clone)]
pub struct BluetoothConnection {
    pub(crate) peripheral: Peripheral,
}

impl BluetoothConnection {
    pub async fn get_props(&self) -> PeripheralProperties {
        self.peripheral.properties().await.unwrap().unwrap()
    }
    
    pub async fn read(&self) -> Result<(), Box<dyn Error>> {
        if !self.peripheral.is_connected().await.unwrap() {
            self.peripheral.connect().await?;
        }

        self.peripheral.discover_services().await?;

        for service in self.peripheral.services() {
            for characteristic in service.characteristics {
                if characteristic.properties.contains(CharPropFlags::NOTIFY) {
                    info!("Subscribing to characteristic {:?}", characteristic.uuid);
                    self.peripheral.subscribe(&characteristic).await?;
                    let mut notification_stream = 
                        self.peripheral.notifications().await?.take(4);
                    while let Some(data) = notification_stream.next().await {
                        info!(
                            "Received data from <somewhere> [{:?}]: {:?}",
                            data.uuid, data.value
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn write(&self, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        if !self.peripheral.is_connected().await.unwrap() {
            self.peripheral.connect().await?;
        }

        self.peripheral.discover_services().await?;
    
        for service in self.peripheral.services() {
            for characteristic in service.characteristics {
                if characteristic.properties.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE) {
                    self.peripheral.write(&characteristic, bytes, WriteType::WithoutResponse).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        if self.peripheral.is_connected().await.expect("Could not get connection status") {
            info!("Disconnecting from peripheral {:?}...", &self.get_props().await.address);
            self.peripheral
                .disconnect()
                .await?;
        }

        Ok(())
    }
}
