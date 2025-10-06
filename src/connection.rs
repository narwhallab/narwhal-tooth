use std::{collections::HashMap, sync::Arc, time::Duration};
use btleplug::{api::{Central as _, CharPropFlags, Characteristic, Peripheral as _, WriteType}, platform::Peripheral};
use futures::StreamExt;
use log::info;
use tokio::{sync::{mpsc::{channel, Sender}, Mutex}, time::timeout};
use uuid::Uuid;
use crate::{device::BluetoothDevice, CENTRAL, GLOBAL_CONNECTION_MANAGER};
use anyhow::{anyhow, Result};

#[derive(Clone)]
pub struct BluetoothConnection {
    pub(crate) peripheral: Peripheral,
    pub(crate) target_characteristic: Characteristic,
    pub(crate) device: BluetoothDevice,
    pub(crate) event_handlers: Arc<Mutex<HashMap<String, Sender<String>>>>,
    pub(crate) connection_uuid: Uuid
}

impl BluetoothConnection {
    async fn is_connected(&self) -> bool {
        return self.peripheral.is_connected().await.unwrap_or(false);
    }

    pub async fn check_alive(&self) -> bool {
        return if let Ok(data) = self.send("NT_CheckAlive".as_bytes()).await {
            data == "Ok"
        } else {
            false
        }
    }

    pub async fn send(&self, payload: &[u8]) -> Result<String> {
        return self.send_with_timeout(payload, Duration::from_secs(60)).await
    }

    pub async fn send_with_timeout(&self, payload: &[u8], duration_timeout: Duration) -> Result<String> {
        if !self.is_connected().await {
            return Err(anyhow!("Peripheral not connected"))
        }

        let uuid = String::from(&Uuid::new_v4().to_string()[..8]);

        let (tx, mut rx) = channel::<String>(1024 * 32);

        self.event_handlers.lock().await.insert(uuid.clone(), tx);

        let mut packet = uuid.as_bytes().to_vec();
        packet.extend_from_slice(&[58]);    // ':'
        packet.extend_from_slice(payload);
        
        self.peripheral.write(&self.target_characteristic, &packet, WriteType::WithResponse).await?;

        info!("Sent to peripheral {:?}", self.device.get_local_name());

        let response_result = timeout(duration_timeout, rx.recv()).await?.ok_or(anyhow!("Timed Out"));

        return response_result
    }

    pub async fn initialize(&self) -> Result<()> {
        if !self.is_connected().await {
            return Err(anyhow!("Peripheral not connected"))
        }

        // Subscribe to the characteristic
        self.peripheral.subscribe(&self.target_characteristic).await?;
        let mut notifications_stream = self.peripheral.notifications().await?;
    
        let event_handlers = self.event_handlers.clone();

        let task = tokio::spawn(async move {
            let event_handlers = event_handlers.clone();

            loop {
                let mut stringified_data = String::new();

                while let Some(notification) = notifications_stream.next().await {
                    let raw_data = notification.value;

                    stringified_data += &String::from_utf8(raw_data).unwrap_or("<Parse Error>".to_string());
                    
                    if stringified_data.ends_with("#;EOF;#") {
                        stringified_data = stringified_data[..stringified_data.len()-7].to_string();
                        break
                    }
                }

                let split_data: Vec<String> = stringified_data.split(":").map(|x| x.to_string()).collect();
                    
                let uuid = split_data[0].clone();
                let payload = split_data[1].clone();

                futures::executor::block_on(async {
                    let mut lock = event_handlers.lock().await;
                    if !lock.contains_key(&uuid) {
                        return;
                    }
                    let tx = lock.remove(&uuid).unwrap();
                    let _res = tx.send(payload).await; // todo: error handling
                });
            }
        });

        info!("Subscribed to peripheral {:?}", self.device.get_local_name());

        let mut global_connection_manager_ref = GLOBAL_CONNECTION_MANAGER.lock().await;
        global_connection_manager_ref.insert(self.connection_uuid, task);

        return Ok(())
    }

    pub async fn terminate(&self) -> Result<()> {
        if !self.is_connected().await {
            return Err(anyhow!("Peripheral not connected"))
        }
        
        // unsubscribe characteristic and terminate handler
        self.peripheral.unsubscribe(&self.target_characteristic).await?;
        if let Some(handle) = GLOBAL_CONNECTION_MANAGER.lock().await.get(&self.connection_uuid) {
            handle.abort();
        }

        info!("Unsubscribed to peripheral {:?}", self.device.get_local_name());

        // disconnect from the peripheral
        self.peripheral.disconnect().await?;
        info!("Disconnected to peripheral {:?}", self.device.get_local_name());

        return Ok(())
    }
}

pub(crate) async fn bluetooth_device_to_peripheral(device: BluetoothDevice, peripherals: Vec<Peripheral>) -> Result<Peripheral> {
    for peripheral in peripherals.iter() {
        let _local_name = peripheral.properties().await?.and_then(|p| p.local_name);
        if Some(device.local_name.clone()) == _local_name {
            return Ok(peripheral.clone())
        }
    }

    return Err(anyhow!("Given peripheral wasn't found"));
}

pub async fn connect_device(device: BluetoothDevice) -> Result<BluetoothConnection>  {
    let central = CENTRAL.get().await;

    let peripherals = central
        .peripherals()
        .await
        .expect("Couldn't retrieve peripherals from BLE adapter...");

    let peripheral = bluetooth_device_to_peripheral(device.clone(), peripherals).await.expect("Invalid Bluetooth Device");

    let is_connected = peripheral.is_connected().await?;

    // attempt peripheral connection
    if !is_connected {
        info!("Connecting to peripheral {:?}...", &device.get_local_name());
        peripheral.connect().await?;
    }

    // failed to connect
    if !peripheral.is_connected().await? {
        return Err(anyhow!("Could not connect to peripheral"));
    }

    peripheral.discover_services().await?;

    for service in peripheral.services() {
        for characteristic in service.characteristics {
            info!("Characteristic Discovered - {:?}", characteristic);
            if characteristic.properties.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE) {
                let connection = BluetoothConnection { 
                    peripheral: peripheral.clone(), 
                    target_characteristic: characteristic,
                    device: device.clone(),
                    event_handlers: Arc::new(Mutex::new(HashMap::new())),
                    connection_uuid: Uuid::new_v4()
                };

                connection.initialize().await?;

                return Ok(connection);
            }
        }
    }

    return Err(anyhow!("Could not find a valid characteristic for read/write"))
}