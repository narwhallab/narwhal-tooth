pub mod bluetooth;
pub mod scan;
pub mod util;

use std::collections::HashMap;
use std::sync::Mutex;
use async_once::AsyncOnce;
use btleplug::platform::Adapter;
use simplelog::{TermLogger, Config};
use lazy_static::lazy_static;
use util::fetch_adapter;
use tokio::sync::mpsc::Sender;

lazy_static! {
    pub static ref CENTRAL: AsyncOnce<Adapter> = AsyncOnce::new(async {
        fetch_adapter().await
    });

    pub static ref EVENT_HANDLER: Mutex<HashMap<String, Sender<String>>> = Mutex::new(HashMap::new());
}

pub fn init_logger() {
    TermLogger::init(log::LevelFilter::Info, Config::default(), simplelog::TerminalMode::Stdout, simplelog::ColorChoice::Auto).unwrap();
}

#[cfg(test)]
mod test {
    use crate::{util::connect_peripheral, scan::scan_bluetooth};

    #[tokio::test]
    async fn test_ble() {
        let result = scan_bluetooth(5).await;
        let hmsoft = result.get_by_name("HMSoft").await.expect("Could not find HMSoft device");
        let connection = connect_peripheral(&hmsoft).await.unwrap();
        connection.initialize().await;

        println!("{}", connection.check_alive().await);
    }
}
