extern crate hidapi;

use abs::abs;
use std::sync::Arc;

use console::style;
use eyre::Result;
use futures_util::{SinkExt, StreamExt};
use hidapi::HidApi;
use if_chain::if_chain;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

const VENDOR_ID: u16 = 1103;
const PEDAL_DIFF: u8 = 20;

fn compare_byte_arrays(a: &[u8], b: &[u8]) -> Vec<usize> {
    if a.len() != b.len() {
        panic!("Byte arrays must be of the same length");
    }

    let mut differing_positions = Vec::new();
    for i in 0..a.len() {
        if a[i] != b[i] {
            differing_positions.push(i);
        }
    }

    differing_positions
}

fn normalise_pedal_value(value: u8) -> f64 {
    (value as f64) / 255.0
}

fn normalise_steer_value(value: u8) -> f64 {
    let value = (value as f64 - 127.0) / 127.0;

    if value.abs() < 0.5 {
        0.0
    } else {
        value
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let debug = std::env::var("DEBUG").is_ok();
    let port = std::env::var("PORT").unwrap_or("8000".to_string());
    let api = HidApi::new()?;
    let mut devices = api.device_list();
    let current_device = devices.find(|device| {
        device.vendor_id() == VENDOR_ID && device.product_string().unwrap_or("").contains("Wheel")
    });

    api.set_open_exclusive(false);

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

    match current_device {
        Some(device_info) => {
            if_chain! {
                if let Some(product_string) = device_info.product_string();
                if let Some(manufacturer_string) = device_info.manufacturer_string();
                then {
                    println!("Device found: {} {}", style(product_string).green(), manufacturer_string);

                    let device = device_info.open_device(&api)?;
                    let device = Arc::new(Mutex::new(device));

                    while let Ok((stream, _)) = listener.accept().await {
                        let device = Arc::clone(&device);
                        let ws_stream = accept_async(stream).await?;
                        let (write_stream, mut read_stream) = ws_stream.split(); // Split the stream
                        let write_stream = Arc::new(Mutex::new(write_stream));
                        let write_stream_clone = Arc::clone(&write_stream);

                        let token = CancellationToken::new();
                        let cloned_token = token.clone();
                        tokio::spawn(async move {
                            let device = device.lock().await;
                            let mut last_buf = [0u8; 29];
                            let mut first_read = true;

                            let mut last_gas: u8 = 0;
                            let mut last_brake: u8 = 0;

                            println!("Connected to client");

                            'main: loop {
                                let mut buf = [0u8; 29];
                                let res = device.read(&mut buf)
                                    .expect("Failed to read from device");

                                tokio::select! {
                                    _ = cloned_token.cancelled() => {
                                        break 'main;
                                    },
                                    _ = tokio::task::yield_now() => {
                                        if last_buf != buf || first_read {
                                            let changed_positions = compare_byte_arrays(&last_buf, &buf);

                                            let buf_array = &buf[..res];
                                            if debug {
                                                let buf_array = buf_array
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(i, &x)| {
                                                        if changed_positions.contains(&i) && !first_read {
                                                            style(format!("{:02X}", x)).red().to_string()
                                                        } else {
                                                            style(format!("{:02X}", x)).green().to_string()
                                                        }
                                                    })
                                                    .collect::<Vec<String>>();

                                                println!("{}", buf_array.join(" "));
                                            }

                                            let steering = buf[3];
                                            let break_pedal = buf[17];
                                            let gas_pedal = buf[18];

                                            let normalised_break_pedal = normalise_pedal_value(break_pedal);
                                            let normalised_gas_pedal = normalise_pedal_value(gas_pedal);
                                            let normalise_steering = normalise_steer_value(steering);
                                            if first_read || abs(gas_pedal, last_gas) > PEDAL_DIFF || abs(break_pedal, last_brake) > PEDAL_DIFF || (gas_pedal != last_gas && gas_pedal == 0) || (break_pedal != last_brake && break_pedal == 0){
                                                let mut write_stream = write_stream_clone.lock().await;
                                                write_stream.send(Message::Text(format!("{:.2} {:.2}", normalised_gas_pedal, normalised_break_pedal))).await.expect("Failed to send message to client");
                                                last_gas = gas_pedal;
                                                last_brake = break_pedal;
                                            }

                                            if debug {
                                                println!("Gas pedal: {:.2}, Break pedal: {:.2}, Steering: {:.2}", normalised_gas_pedal, normalised_break_pedal, normalise_steering);
                                            }

                                            last_buf = buf;
                                            first_read = false;
                                        };
                                    }
                                }

                            }
                        });

                        while let Some(Ok(msg)) = read_stream.next().await {
                            match msg {
                                Message::Text(text) => {
                                    println!("Received a message: {}", text);
                                },
                                Message::Close(_) => {
                                    break;
                                },
                                _ => {}
                            }
                        }

                        token.cancel();
                        println!("Connection closed");
                    }
                }
            }
        }
        None => {
            println!(
                "{}",
                style("Device not found, please make sure the device is connected.").red()
            );
        }
    }

    Ok(())
}
