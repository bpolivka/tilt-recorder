use btleplug::api::{Central, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::Manager;
use chrono::{DateTime, Utc};
use futures::stream::{select_all, StreamExt};
use influxdb::{Client, InfluxDbWriteable};
use std::env;
use std::error::Error;

const TILT_MFG_ID: u16 = 0x4c;

#[derive(Debug)]
enum TiltColor {
    Red,
    Green,
    Black,
    Purple,
    Orange,
    Blue,
    Yellow,
    Pink,
}

fn code_to_color(code: u8) -> Option<TiltColor> {
    match code {
        0x10 => Some(TiltColor::Red),
        0x20 => Some(TiltColor::Green),
        0x30 => Some(TiltColor::Black),
        0x40 => Some(TiltColor::Purple),
        0x50 => Some(TiltColor::Orange),
        0x60 => Some(TiltColor::Blue),
        0x70 => Some(TiltColor::Yellow),
        0x80 => Some(TiltColor::Pink),
        _ => None,
    }
}

fn color_name(color: TiltColor) -> &'static str {
    match color {
        TiltColor::Red => "red",
        TiltColor::Green => "green",
        TiltColor::Black => "black",
        TiltColor::Purple => "purple",
        TiltColor::Orange => "orange",
        TiltColor::Blue => "blue",
        TiltColor::Yellow => "yellow",
        TiltColor::Pink => "pink",
    }
}

#[derive(InfluxDbWriteable, Debug)]
struct TiltReading {
    time: DateTime<Utc>,
    #[influxdb(tag)]
    color: &'static str,
    temp: u16,
    sg: f32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new(env::var("INFLUXDB_URL").unwrap(), "brewery");

    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;

    let mut streams = Vec::new();

    for adapter in adapters.iter() {
        let events = adapter.events().await?;
        adapter.start_scan(ScanFilter::default()).await?;
        streams.push(events);
        println!("Scanning {:?}", adapter);
    }

    let mut events = select_all(streams);

    while let Some(event) = events.next().await {
        match event {
            CentralEvent::ManufacturerDataAdvertisement {
                manufacturer_data, ..
            } => {
                if let Some(data) = manufacturer_data.get(&TILT_MFG_ID) {
                    if !is_tilt_update(data) {
                        continue;
                    }

                    let len = data.len();
                    let color = code_to_color(data[5]).unwrap();
                    let temp = ((data[len - 5] as u16) << 8) | (data[len - 4] as u16);
                    let sg_int = ((data[len - 3] as u16) << 8) | (data[len - 2] as u16);
                    let sg = (sg_int as f32) / 1000.0;

                    let reading = TiltReading {
                        time: Utc::now(),
                        color: color_name(color),
                        temp: temp,
                        sg: sg,
                    };

                    println!("tilt: {:?}", reading);

                    let _ = client.query(&reading.into_query("tilt")).await;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn is_tilt_update(data: &Vec<u8>) -> bool {
    if data.len() < 20 {
        return false;
    }

    if &data[0..2] != &[0x2, 0x15] {
        return false;
    }

    // UUID: A4 95 BB <color> C5 B1 4B 44 B5 12 13 70 F0 2D 74 DE

    if &data[2..5] != &[0xa4, 0x95, 0xbb]
        || &data[6..18]
            != &[
                0xc5, 0xb1, 0x4b, 0x44, 0xb5, 0x12, 0x13, 0x70, 0xf0, 0x2d, 0x74, 0xde,
            ]
    {
        return false;
    }

    true
}
