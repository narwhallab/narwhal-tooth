use btleplug::{api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter}, platform::{Adapter, Manager}};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::CENTRAL;

#[derive(Clone)]
pub struct BluetoothDevice {
    pub addr: String,
    pub local_name: Option<String>
}

impl BluetoothDevice {
    pub fn get_local_name(&self) -> Option<String> {
        self.local_name.clone()
    }

    pub fn get_addr(&self) -> String {
        self.addr.clone()
    }
}

pub async fn get_central() -> anyhow::Result<Adapter> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    adapters.into_iter().nth(0).ok_or(anyhow::anyhow!("No bluetooth adapter was found"))
}

pub async fn attempt_connection_by_name(peripheral_name: &str, tx: mpsc::Sender<BluetoothDevice>) -> anyhow::Result<()> {
    let central = CENTRAL.get().await;

    let adapter_state = central.adapter_state().await?;
    println!("Adapter State: {:?}", adapter_state);

    let mut events = central.events().await?;

    central.start_scan(ScanFilter::default()).await?;

    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(peripheral_id) => {
                let peripheral = central.peripheral(&peripheral_id).await?;
                if let Some(properties) = peripheral.properties().await? {
                    let _peripheral_address = properties.address.to_string();

                    if let Some(_peripheral_name) = properties.local_name {
                        println!("Discovered: {}", &_peripheral_name);
                        if &_peripheral_name == peripheral_name {
                            let bluetooth_device = BluetoothDevice {
                                local_name: Some(_peripheral_name),
                                addr: peripheral_name.to_string()
                            };

                            tx.send(bluetooth_device).await?;

                            return Ok(())
                        }

                    }
                }
            },
            _ => {}
        }
    }


    Err(anyhow::anyhow!("Couldn't find matching peripheral"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::sync::mpsc::channel;

    use crate::device::attempt_connection_by_name;

    #[tokio::test]
    async fn test_ble_connection() {
        let (tx, mut rx) = channel(1);

        tokio::spawn(async move {
            attempt_connection_by_name("HMSoft", tx).await
        });

        match tokio::time::timeout(Duration::from_millis(10000), rx.recv()).await {
            Ok(Some(msg)) => println!("Got: {}", msg.get_addr()),
            Ok(None) => println!("Channel closed"),
            Err(_) => println!("Timed out"),
        }
    }
}