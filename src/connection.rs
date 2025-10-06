use std::{collections::HashMap, error::Error, sync::Arc, time::Duration};
use btleplug::{api::{Central as _, CharPropFlags, Characteristic, Peripheral as _, WriteType}, platform::Peripheral};
use futures::StreamExt;
use log::info;
use tokio::{sync::{mpsc::{channel, Sender}, Mutex}, task::JoinHandle, time::timeout};
use uuid::Uuid;
use crate::{device::BluetoothDevice, CENTRAL};

#[derive(Clone)]
pub struct BluetoothConnection {
    pub(crate) peripheral: Peripheral,
    pub(crate) target_characteristic: Characteristic,
    pub(crate) device: BluetoothDevice,
    pub(crate) event_handlers: Arc<Mutex<HashMap<String, Sender<String>>>>
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

    pub async fn send(&self, payload: &[u8]) -> Result<String, Box<dyn Error>> {
        return self.send_with_timeout(payload, Duration::from_secs(60)).await
    }

    pub async fn send_with_timeout(&self, payload: &[u8], duration_timeout: Duration) -> Result<String, Box<dyn Error>> {
        if !self.is_connected().await {
            return Err("Peripheral not connected".into())
        }

        let uuid = String::from(&Uuid::new_v4().to_string()[..8]);

        let (tx, mut rx) = channel::<String>(1024 * 32);

        self.event_handlers.lock().await.insert(uuid.clone(), tx);

        let mut packet = uuid.as_bytes().to_vec();
        packet.extend_from_slice(&[58]);    // ':'
        packet.extend_from_slice(payload);
        
        self.peripheral.write(&self.target_characteristic, &packet, WriteType::WithResponse).await?;

        info!("Sent to peripheral {:?}", self.device.get_local_name());

        let response_result = timeout(duration_timeout, rx.recv()).await?.ok_or("Timed Out".into());

        return response_result
    }

    pub async fn initialize(&self) -> anyhow::Result<JoinHandle<()>> {
        if !self.is_connected().await {
            return Err(anyhow::anyhow!("Peripheral not connected"))
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

        return Ok(task)
    }

    pub async fn terminate(&self, join_handle: JoinHandle<()>) -> Result<(), Box<dyn Error>> {
        if !self.is_connected().await {
            return Err("Peripheral not connected".into())
        }
        
        // unsubscribe characteristic and terminate handler
        self.peripheral.unsubscribe(&self.target_characteristic).await?;
        join_handle.abort();
        info!("Unsubscribed to peripheral {:?}", self.device.get_local_name());

        // disconnect from the peripheral
        self.peripheral.disconnect().await?;
        info!("Disconnected to peripheral {:?}", self.device.get_local_name());

        return Ok(())
    }
}

pub(crate) async fn bluetooth_device_to_peripheral(device: BluetoothDevice, peripherals: Vec<Peripheral>) -> anyhow::Result<Peripheral> {
    for peripheral in peripherals.iter() {
        let _local_name = peripheral.properties().await?.and_then(|p| p.local_name);
        if Some(device.local_name.clone()) == _local_name {
            return Ok(peripheral.clone())
        }
    }

    return Err(anyhow::anyhow!("Given peripheral wasn't found"));
}

pub async fn connect_device(device: BluetoothDevice) -> anyhow::Result<(BluetoothConnection, JoinHandle<()>)>  {
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
        return Err(anyhow::anyhow!("Could not connect to peripheral"));
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
                    event_handlers: Arc::new(Mutex::new(HashMap::new()))
                };

                let handler = connection.initialize().await?;

                return Ok((connection, handler));
            }
        }
    }

    return Err(anyhow::anyhow!("Could not find a valid characteristic for read/write"))
}