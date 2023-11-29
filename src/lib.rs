pub mod bluetooth;
pub mod scan;
pub mod util;

use std::collections::HashMap;
use std::sync::Mutex;
use async_once::AsyncOnce;
use btleplug::platform::Adapter;
use lazy_static::lazy_static;
use util::fetch_adapter;
use tokio::sync::mpsc::Sender;

lazy_static! {
    pub static ref CENTRAL: AsyncOnce<Adapter> = AsyncOnce::new(async {
        fetch_adapter().await
    });

    pub static ref EVENT_HANDLER: Mutex<HashMap<String, Sender<String>>> = Mutex::new(HashMap::new());
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::{util::connect_device, scan::scan_bluetooth};

    #[tokio::test]
    async fn test_ble() {
        let result = scan_bluetooth(Duration::from_secs(3)).await;
        let hmsoft = result.search_by_name("HMSoft".to_string()).await.expect("Could not find device");
        let connection = connect_device(hmsoft).await.unwrap();

        println!("{}", connection.check_alive().await);
    }
}
