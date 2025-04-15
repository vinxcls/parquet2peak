# 🛠️ blf2parquet & parquet2peak

Rust-based tools to convert `.blf` files into `.parquet` format and replay them on a CAN bus via a PEAK interface.

## 📦 Description

- **`blf2parquet`**: Converts a `.blf` file (Binary Logging Format) into a `.parquet` file.
- **`parquet2peak`**: Reads a `.parquet` file and sends its CAN frames over the bus using a PEAK-compatible interface.

---

## 📦 Dependencies

This project depends on the **PEAK** library to interface with the CAN bus. Please follow these steps to install and configure the necessary dependencies:

1. **Download the PEAK Library**
   Visit the official website of PEAK and download the necessary drivers and libraries:
   [PEAK System](https://www.peak-system.com/)

2. **Configure the `build.rs` File*
   Once the PEAK library is downloaded and installed, copy the `PCANBasic.lib` in `C:\Peak`.
   If you prefer to use another path, you need to modify the `build.rs` file to properly link the library in your Rust project.

   The `build.rs` file should include the following code:

   ```rust
   fn main() {
       println!("cargo:rustc-link-search=native=C:\\Peak"); // Library path
       println!("cargo:rustc-link-lib=static=PCANBasic"); // Link to static library
   }

## 🚀 Build Instructions

Build the binaries in `release` mode using the following command:

```
cargo build --release
```
The resulting executables will be located in `target/release/`.

## ⚙️ Usage

### blf2parquet

**Usage**:
```
Usage: blf2parquet.exe [OPTIONS] --input <INPUT> --output <OUTPUT>

Options:
  -i, --input <INPUT>                        Blf input file
  -o, --output <OUTPUT>                      Parquet output file
  -c, --channel <CHANNEL>                    Channel [default: 0]
  -s, --start-percentage <START_PERCENTAGE>  Start percentage [default: 0]
  -e, --end-percentage <END_PERCENTAGE>      End percentage [default: 100]
  -h, --help                                 Print help
  -V, --version                              Print version
```
**Example**:
```
blf2parquet.exe -i input.blf -o output.parquet -c 0 -s 50 -e 70
```
This command converts `input.blf` into `output.parquet` using channel `0`, starting at `50%` and ending at `70%` of the file's duration.

### parquet2peak

**Usage**:
```
Usage: parquet2peak.exe [OPTIONS] --file <FILE>

Options:
  -f, --file <FILE>                File path
  -l, --loop-forever               Enable infinite loop
  -e, --exclude-id <EXCLUDE_ID>    Exclusion ID list in hex (eg: "0x0A,0x0B,0x1F") [default: ]
  -u, --usb-can-bus <USB_CAN_BUS>  Bus USB CAN: from 1 to 16 [default: 1]
  -h, --help                       Print help
  -V, --version                    Print version
```
- `forever`: set to `1` to send in a loop, or `0` (default) for one-shot sending
- `exclude_can_id_list`: optional comma-separated list of CAN IDs to exclude (e.g. `0x1,0x7FF`)

**Example**:
```
parquet2peak.exe -f output.parquet -l -e 0x1,0x7ff -u 10
```
This command replays `output.parquet` continuously on USB CAN bus n.10, excluding CAN IDs `0x1` and `0x7FF`.

## ✅ Testing
There are no automated tests yet. To validate manually:

- Compare the original .blf file to the generated .parquet
- Monitor the CAN bus with a compatible sniffer during replay

## 📄 License
This project is licensed under the MIT License – see the [LICENSE](./LICENSE) file for details.

## 🤝 Contributing
Pull requests, feature suggestions, and bug reports are welcome!

## 📫 Contact
Feel free to open an issue or contact the author directly for support or questions.

