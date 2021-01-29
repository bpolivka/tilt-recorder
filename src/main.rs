use btleplug::{
    api::{Central, CentralEvent, Peripheral},
    bluez::manager::Manager,
};
use chrono::{DateTime, Utc};
use env::var;
use influxdb::{Client, InfluxDbWriteable};
use std::env;

const TILT_MFG_ID: [u8; 6] = [0x4c, 0x00, 0x02, 0x15, 0xa4, 0x95];

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
    #[tag]
    color: &'static str,
    temp: u16,
    sg: f32,
}

#[async_std::main]
async fn main() {
    let manager = Manager::new().unwrap();

    let client = Client::new(env::var("INFLUXDB_URL").unwrap(), "brewery");

    let adapters = manager.adapters().unwrap();
    let adapter = adapters.into_iter().nth(0).unwrap();

    let central = adapter.connect().unwrap();

    let event_receiver = central.event_receiver().unwrap();

    central.start_scan().unwrap();

    while let Ok(event) = event_receiver.recv() {
        match event {
            CentralEvent::DeviceUpdated(bd_addr) => {
                let peripheral = central.peripheral(bd_addr).unwrap();
                match peripheral.properties().manufacturer_data {
                    Some(data) => {
                        let len = data.len();
                        if len >= 6 && &data[0..6] == TILT_MFG_ID {
                            let color = code_to_color(data[7]).unwrap();
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
                    None => {}
                }
            }
            _ => {}
        }
    }
}
