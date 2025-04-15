use std::{
    fs::File,
    io::Write,
    path::Path,
    thread::sleep,
    time::{Duration, Instant},
};
use parquet::{
    file::reader::{FileReader, SerializedFileReader},
    record::{Field, Row, RowAccessor},
    errors::ParquetError,
};
use peak_can::{
    bus::UsbBus,
    socket::{
        Baudrate, CanFrame, FrameConstructionError, MessageType, SendCan,
        usb::UsbCanSocket,
    },
};
use clap::Parser;

fn process_row(row: &Row) -> Result<(f64, u32, Vec<u8>), ParquetError> {
    let mut data = Vec::new();

    let timing = row.get_double(0)?;
    let id = row.get_uint(1)? as u32;
    if let Ok(list) = row.get_list(2) {
        for f in list.elements().iter() {
            if let Field::UByte(value) = f {
                data.push(*value as u8);
            }
        }
    }

    Ok((timing, id, data))
}

fn send_can_messages(content: &[(f64, u32, Vec<u8>)], socket: &UsbCanSocket) -> Result<(), FrameConstructionError> {
    let mut old_timing: Option<f64> = None;
    let mut passive_timing = Duration::new(0, 0);
    let mut c = 0;
    let mut old_perc = 0.0;
    let content_size = content.len() as f64;
    let print_interval = Duration::from_millis(40);
    let mut last_print_time = Instant::now();

    for (curr, id, can_data) in content {
        if let Some(previous) = old_timing {
            let diff = ((*curr - previous).max(0.0) * 1_000_000_000.0) - (passive_timing.as_nanos() as f64);
            let udiff = (diff / 1_000.0) as u64;
            sleep(Duration::from_micros(udiff));
            //println!("Waiting {}us", udiff);
        }

        let start = Instant::now();
        old_timing = Some(*curr);

        let t = if *id < 0x800 {
            MessageType::Standard
        } else {
            MessageType::Extended
        };

        let frame = CanFrame::new(*id, t, can_data)?;

        if let Err(err) = socket.send(frame) {
            eprintln!("Error {:?}: unable to send frame {:?}", err, frame);
            break;
        }

        c += 1;
        if last_print_time.elapsed() >= print_interval {
            let perc = (c as f64 / content_size) * 100.0;
            if perc >= (old_perc + 0.01) {
                old_perc = perc;
                print!("\r[{:.2}%]", perc);
                std::io::stdout().flush().unwrap();
            }
            last_print_time = Instant::now();
        }
        passive_timing = start.elapsed();
    }
    print!("\r[{:.2}%]", (c as f64 / content_size) * 100.0);

    Ok(())
}

fn parse_hex_list(input: Option<String>) -> Vec<u32> {
    input.unwrap_or_default()
         .split(',')
         .filter_map(|num| num.trim().strip_prefix("0x")
         .and_then(|hex| u32::from_str_radix(hex, 16).ok()))
         .collect()
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// File path
    #[arg(short,long)]
    file: String,

    /// Enable infinite loop
    #[arg(short, long, default_value_t = false)]
    loop_forever: bool,

    /// Exclusion ID list in hex (eg: "0x0A,0x0B,0x1F")
    #[arg(short, long, default_value = "")]
    exclude_id: Option<String>,

    /// Bus USB CAN: from 1 to 16
    #[arg(short, long, default_value_t = 1)]
    usb_can_bus: u16,
}

fn main() -> parquet::errors::Result<()> {
    let args = Args::parse();

    let file_path = &Path::new(&args.file);
    let forever = args.loop_forever;
    let exclude_id = parse_hex_list(args.exclude_id);
    let usb_can_bus = UsbBus::try_from(args.usb_can_bus).unwrap_or_else(|_| {
        eprintln!("Invalid can bus resetting to USB1!");
        UsbBus::USB1
    });

    if exclude_id.is_empty() == false {
        print!("Apply filter: {:?}", exclude_id);
    }

    let start = Instant::now();
    // Apri il file Parquet
    let file = File::open(file_path)?;
    let reader = SerializedFileReader::new(file).unwrap();

    let mut row_iter = reader.get_row_iter(None).unwrap();

    let mut content: Vec<(f64, u32, Vec<u8>)> = Vec::new();
    let mut elem = 0;
    let mut felem = 0;

    while let Some(Ok(row)) = row_iter.next() {
        if let Ok((timing, id, data)) = process_row(&row) {
            if ! exclude_id.contains(&id) {
                content.push((timing, id, data));
                felem += 1;
            }
        }
        elem += 1;
    }

    let duration = start.elapsed();
    println!("Loading data ({} of {}) from {:?}: {:?}", felem, elem, file_path,
             duration);

    let usb_socket = match UsbCanSocket::open(usb_can_bus, Baudrate::Baud500K) {
        Ok(socket) => socket,
        Err(err) => {
            println!("Unable to open USB socket: {:?}", err);
            return Ok(());
        }
    };

    println!("Starting simulation of {} frames (loop:{}, Bus:{:?})",
             content.len(), forever, usb_can_bus);

    loop {
        if let Err(_) = send_can_messages(&content, &usb_socket) {
            println!("Error sending CAN frames.");
            break;
        }
        if forever == false {
            break;
        }
        println!("Restarting...");
    }
    println!("Exit!!!");

    Ok(())
}

