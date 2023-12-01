# Narwhal-Tooth
Bluetooth communication between PC and Arduino

### How to use
```rust
let result = scan_bluetooth(Duration::from_secs(3)).await; // scan for bluetooth devices for 3 seconds
let device = result.search_by_name("MyDevice".to_string()).await.unwrap(); // scan for a device with a matching name
let connection = connect_device(device).await.unwrap();

println!("{}", connection.check_alive().await); // sends 'NT_CheckAlive' to Arduino. returns true if response is 'Ok'

connection.disconnect().await.unwrap(); // disconnect
```

### Protocol
Ping Pong with a simple packet. All packets should be in the following format. ***First 8 letters of a random UUID*** goes in the `<uuid>` and ***the data you want to send*** goes in the `<payload>`
```
<uuid>:<payload>
```