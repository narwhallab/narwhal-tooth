# Narwhal-Tooth
Bluetooth communication between PC and Arduino

### How to use
```rust
// tx, rx to retreieve scan results
let (tx, mut rx) = channel(1);

// pass the tx to the scan_device_by_name function.
// BluetoothDevice struct will be sent to the rx when device was found
tokio::spawn(async move {
    scan_device_by_name("HMSoft", tx).await
});

// when the matching BluetoothDevice is sent, you can now connect to it
// you can add a timeout here, too.
if let Ok(Some(device)) = tokio::time::timeout(Duration::from_millis(10000), rx.recv()).await {
    
    // connect to the bluetooth device
    let (connection, handle) = connect_device(device).await.unwrap();

    // send a message to the device and print out the response
    let res = connection.send(b"Hello How are you?").await.unwrap();
    println!("Bluetooth says: {}", res);

    // terminate the connection
    connection.terminate(handle).await.unwrap();
}
```

### Protocol
Ping Pong with a simple packet. All packets should be in the following format. ***First 8 letters of a random UUID*** goes in the `<uuid>` and ***the data you want to send*** goes in the `<payload>`
```
<uuid>:<payload>
```