use async_once::AsyncOnce;
use btleplug::{api::Manager as _, platform::{Adapter, Manager}};
use lazy_static::lazy_static;

pub mod connection;
pub mod device;

lazy_static! {
    pub static ref CENTRAL: AsyncOnce<Adapter> = AsyncOnce::new(async {
        get_central().await.unwrap()
    });
}

pub async fn get_central() -> anyhow::Result<Adapter> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    adapters.into_iter().nth(0).ok_or(anyhow::anyhow!("No bluetooth adapter was found"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{sync::mpsc::channel, time::Instant};

    use crate::{connection::connect_device, device::scan_device_by_name};

    #[tokio::test]
    async fn test_communication() {
        let (tx, mut rx) = channel(1);

        tokio::spawn(async move {
            scan_device_by_name("HMSoft", tx).await
        });

        if let Ok(Some(device)) = tokio::time::timeout(Duration::from_millis(10000), rx.recv()).await {
            let (connection, handle) = connect_device(device).await.unwrap();
            
            for i in 0..5 {
                let mut data = vec![];
                for _ in 0..15 {
                    let start = Instant::now();
                    let a1 = start.elapsed().as_micros() as i128;
                    let res = connection.send(b"Hello How are you?").await.unwrap();
                    let b1 = &res[res.len()-8..].parse::<i128>().unwrap();

                    let res = connection.send(b"Hello How are you?").await.unwrap();
                    let a2 = start.elapsed().as_micros() as i128;
                    let b2 = &res[res.len()-8..].parse::<i128>().unwrap();


                    // println!("BT: {}", b2 - b1);
                    // println!("RST: {}", a2 - a1);
                    // println!("DIFF: {}", b2 - b1 - a2 + a1);

                    data.push((b2 - b1 - a2 + a1) as f64);
                }
                let sum: f64 = data.iter().sum();
                let mean = sum / data.len() as f64;

                // compute variance
                let variance = data.iter()
                    .map(|value| {
                        let diff = mean - *value;
                        diff * diff
                    })
                    .sum::<f64>() / data.len() as f64;
                println!("[{}] Mean: {} / Variance: {}", i, mean, variance);
            }

            connection.terminate(handle).await.unwrap();
        }
    }
}