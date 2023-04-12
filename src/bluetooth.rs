use std::{error::Error, str::FromStr};
use btleplug::{platform::{Adapter, Manager, Peripheral}, api::{Manager as _, Central as _, CharPropFlags, WriteType, Peripheral as _, BDAddr, PeripheralProperties, ValueNotification}};
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
    let peripheral = CENTRAL.get()
        .await
        .peripherals()
        .await?
        .iter()
        .find(|p| p.address().eq(&BDAddr::from_str(addr).unwrap()))
        .expect("Error")
        .to_owned();

    Ok(peripheral)
}

#[derive(Clone)]
pub struct BluetoothConnection {
    pub(crate) peripheral: Peripheral,
    pub(crate) target_characteristic: btleplug::api::Characteristic,
}

impl BluetoothConnection {
    pub async fn valid(&self) -> bool {
        self.peripheral.is_connected().await.unwrap()
    }

    pub async fn get_props(&self) -> PeripheralProperties {
        self.peripheral.properties().await.unwrap().unwrap()
    }

    pub async fn unsubscribe(&self) -> Result<(), Box<dyn Error>> {
        if !self.valid().await {
            return Err("Peripheral not connected".into())
        }

        self.peripheral.unsubscribe(&self.target_characteristic).await?;

        Ok(())
    }
    
    pub async fn subscribe(&self, handle: fn(ValueNotification) -> ()) -> Result<(), Box<dyn Error>> {
        if !self.valid().await {
            return Err("Peripheral not connected".into())
        }

        self.peripheral.subscribe(&self.target_characteristic).await?;

        let mut stream = self.peripheral.notifications().await?;

        tokio::spawn(async move {
            while let Some(data) = stream.next().await {
                handle(data);
            }
        });

        Ok(())
    }

    pub async fn write(&self, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        if !self.valid().await {
            return Err("Peripheral not connected".into())
        }

        self.peripheral.write(&self.target_characteristic, bytes, WriteType::WithoutResponse).await?;

        Ok(())
    }

    pub async fn read(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        if !self.valid().await {
            return Err("Peripheral not connected".into())
        }

        let result = self.peripheral.read(&self.target_characteristic).await?;

        Ok(result)
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        if self.valid().await {
            self.peripheral.disconnect().await?;
        }

        Ok(())
    }
}
