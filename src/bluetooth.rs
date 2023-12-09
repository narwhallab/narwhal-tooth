use std::{error::Error, time::Duration};
use btleplug::{platform::Peripheral, api::{Peripheral as _, Characteristic, WriteType}};
use futures::StreamExt;
use log::info;
use tokio::{task::JoinHandle, sync::mpsc::channel, time::timeout};
use uuid::Uuid;
use crate::{EVENT_HANDLER, scan::BluetoothDevice};

#[derive(Clone)]
pub struct BluetoothConnection {
    pub(crate) peripheral: Peripheral,
    pub(crate) target_characteristic: Characteristic,
    pub(crate) device: BluetoothDevice
}

impl BluetoothConnection {
    async fn is_api_available(&self) -> bool {
        return self.peripheral.is_connected().await.unwrap();
    }

    pub(crate) async fn initialize(&self) {
        let _ = self.subscribe(|(uuid, data)| {
            futures::executor::block_on(async move {
                let mut lock = EVENT_HANDLER.lock().await;
                if !lock.contains_key(&uuid) {
                    return;
                }
                let tx = lock.remove(&uuid).unwrap();
                let _res = tx.send(data).await; // todo: maybe this might be useful?
            });
        }).await;
    }

    pub async fn check_alive(&self) -> bool {
        return if let Ok(data) = self.send("NT_CheckAlive".as_bytes()).await {
            data == "Ok"
        } else {
            false
        }
    }

    pub async fn send(&self, payload: &[u8]) -> Result<String, Box<dyn Error>> {
        return self.send_with_timeout(payload, Duration::from_secs(5)).await
    }

    pub async fn send_with_timeout(&self, payload: &[u8], duration_timeout: Duration) -> Result<String, Box<dyn Error>> {
        if !self.is_api_available().await {
            return Err("Peripheral not connected".into())
        }

        let uuid = String::from(&Uuid::new_v4().to_string()[..8]);

        let (tx, mut rx) = channel::<String>(1024);

        EVENT_HANDLER.lock().await.insert(uuid.clone(), tx);

        let mut packet = uuid.as_bytes().to_vec();
        packet.extend_from_slice(&[58]);    // ':'
        packet.extend_from_slice(payload);
        
        self.peripheral.write(&self.target_characteristic, &packet, WriteType::WithResponse).await?;

        info!("Sent to peripheral {}", self.get_local_name().await);

        let response_result = timeout(duration_timeout, rx.recv()).await?.ok_or("Timed Out".into());

        return response_result
    }
    
    // todo: warning: not sure what will happen if resubscribed
    pub async fn subscribe<T: FnMut((String, String)) -> () + Send + 'static>(&self, mut handle: T) -> Result<JoinHandle<()>, Box<dyn Error>> {
        if !self.is_api_available().await {
            return Err("Peripheral not connected".into())
        }

        self.peripheral.subscribe(&self.target_characteristic).await?;

        let mut notifications_stream = self.peripheral.notifications().await?;

        let task = tokio::spawn(async move {
            while let Some(notification) = notifications_stream.next().await {
                let raw_data = notification.value;

                let stringified_data = String::from_utf8(raw_data).unwrap_or("<Parse Error>".to_string());
                let split_data: Vec<String> = stringified_data.split(":").map(|x| x.to_string()).collect();
                
                let uuid = split_data[0].clone();
                let payload = split_data[1].clone();
                
                handle((uuid, payload));
            }
        });

        info!("Subscribed to peripheral {}", self.get_local_name().await);

        return Ok(task)
    }

    pub async fn unsubscribe(&self) -> Result<(), Box<dyn Error>> {
        if !self.is_api_available().await {
            return Err("Peripheral not connected".into())
        }
        
        self.peripheral.unsubscribe(&self.target_characteristic).await?;

        info!("Unsubscribed to peripheral {}", self.get_local_name().await);

        return Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        if self.is_api_available().await {
            self.peripheral.disconnect().await?;

            info!("Disconnected to peripheral {}", self.get_local_name().await);
        }

        return Ok(())
    }

    pub async fn get_local_name(&self) -> String {
        return self.device.get_local_name().unwrap_or("<no name>".to_string())
    }
}
