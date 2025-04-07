use ablf::BlfFile;
use ablf::ObjectTypes;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Instant;
use std::time::Duration;
use chrono::{TimeZone, Utc};
use arrow::buffer::OffsetBuffer;
use arrow::array::ArrayRef;
use arrow::array::UInt8Array;
use arrow::array::UInt32Array;
use arrow::array::Float64Array;
use arrow::array::LargeListArray;
use arrow::record_batch::RecordBatch;
use arrow::datatypes::DataType;
use arrow::datatypes::Field;
use arrow::datatypes::Schema;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input blf> <output parquet> <channel> [start percentage] [end percentage]", args[0]);
        std::process::exit(1);
    }

    let input_blf = &args[1];
    let output_parquet = &args[2];
    let channel: u16 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0) + 1;
    let start_percentage: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let end_percentage: f64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(100.0);

    let start = Instant::now();
    let in_file = match File::open(input_blf) {
        Ok(file) => file,
        Err(error) => {
            println!("Error opening {}: {:?}", input_blf, error);
            std::process::exit(1);
        }
    };
    let reader = BufReader::new(in_file);
    let blf = match BlfFile::from_reader(reader) {
        Ok(b) => b,
        Err((error, _)) => {
            eprintln!("Error in BLF file: {:?}", error);
            std::process::exit(1);
        }
    };
    let duration = start.elapsed();
    println!("Convert from file: {:?}", duration);

    let mut vts: Vec<f64> = Vec::new();
    let mut vid: Vec<u32> = Vec::new();
    let mut vdata: Vec<u8> = Vec::new();
    let mut vlen: Vec<usize> = Vec::new();
    let mut c = 0;

    let objects = blf.file_stats.object_count;

    let dt = blf.file_stats.measurement_start_time().expect("Invalid datetime");
    let start_timestamp = Utc.from_utc_datetime(&dt);

    println!("Filtering {} on channel {} and from {}% to {}%", objects, channel - 1,
             start_percentage, end_percentage);

    let blf_iter = blf.into_iter();

    for (_, obj) in blf_iter.enumerate() {
        c += 1;
        let perc = ((c as f64) / (objects as f64)) * 100.0;
        if perc < start_percentage {
            continue;
        }
        if perc > end_percentage {
            break;
        }
        //print!("\r[{:.2}%]", perc);
        match obj.data {
            ObjectTypes::CanMessage86(ref can_msg) => {
                let ts = start_timestamp + if can_msg.header.flags == 1 {
                            Duration::from_millis(can_msg.header.timestamp_ns)
                        } else {
                            Duration::from_nanos(can_msg.header.timestamp_ns)
                        };
                let ch = can_msg.channel;
                let id = can_msg.id & 0x1FFFFFFF;
                let data = &can_msg.data;
                if ch == channel {
                    let tsf = ts.timestamp() as f64 + (ts.timestamp_subsec_nanos() as f64 / 1e9);
                    vts.push(tsf);
                    vid.push(id);
                    vdata.extend_from_slice(data);
                    vlen.push(data.len());
                    //print!("ts={} id={} data=", tsf, id);
                    //for byte in data {
                    //    print!("0x{:02x},", byte);
                    //}
                    //println!();
                }
            }
            _ => { }
        }
    }

    let schema = Schema::new(vec![
        Field::new("ts", DataType::Float64, false),
        Field::new("id", DataType::UInt32, false),
        Field::new("data", DataType::LargeList(Arc::new(Field::new_list_field(DataType::UInt8, true))), false),
    ]);

    let vts_array: ArrayRef = Arc::new(Float64Array::from(vts));
    let vid_array: ArrayRef = Arc::new(UInt32Array::from(vid));

    let vdata_array: ArrayRef = Arc::new(
                                    LargeListArray::try_new(
                                        Arc::new(Field::new_list_field(DataType::UInt8, true)),
                                        OffsetBuffer::<i64>::from_lengths(vlen),
                                        Arc::new(UInt8Array::from(vdata)), None).unwrap());
    let batch = RecordBatch::try_new(Arc::new(schema),
                vec![Arc::new(vts_array), Arc::new(vid_array), Arc::new(vdata_array)]).unwrap();

    let duration = start.elapsed();
    println!("Convert to records {}: {:?}", batch.num_rows(), duration);

    let out_file = match File::create(output_parquet) {
        Ok(file) => file,
        Err(error) => {
            println!("Error opening {}: {:?}", output_parquet, error);
            std::process::exit(1);
        }
    };
    let props = WriterProperties::builder().set_compression(Compression::SNAPPY)
                                           .build();

    let mut writer = ArrowWriter::try_new(out_file, batch.schema(), Some(props)).unwrap();

    writer.write(&batch).expect("Writing batch");

    // writer must be closed to write footer
    writer.close().unwrap();

    let duration = start.elapsed();
    println!("Total execution time: {:?}", duration);

    Ok(())
}
