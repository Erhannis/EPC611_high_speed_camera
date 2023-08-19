extern crate serialport;

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::time::Duration;

use serialport::SerialPort;

const FRAME_SX: usize = 8;
const FRAME_SY: usize = 8;
const FRAME_BYTES: usize = FRAME_SX*FRAME_SY*2;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut port = serialport::new("/dev/ttyUSB0", 115_200)
        .timeout(Duration::from_millis(1000))
        .open().expect("Failed to open port");


    // let output = "This is a test. This is only a test.".as_bytes();
    // port.write(output).expect("Write failed!");

    let mut buf: VecDeque<u8> = VecDeque::new();

    let mut serial_buf: Vec<u8> = vec![0; 128+3];
    port.read_exact(serial_buf.as_mut_slice()).expect("Found no data!");
    buf.write_all(&serial_buf).expect("Failed to copy?");

    while buf.len() >= 3 && !(buf[0] == b'F' && buf[1] == b'R' && buf[2] == b'\n') {
        buf.pop_front();
    }
    port.read_exact(serial_buf.as_mut_slice()).expect("Found no data!");
    buf.write_all(&serial_buf).expect("Failed to copy?");
    let frameBytes: Vec<u8> = buf.drain(0..FRAME_BYTES).collect();

    //LEAK This could be slow - though probably not enough to matter.
    let frame: Vec<i16> = frameBytes
        .chunks_exact(2)
        .into_iter()
        .map(|a| i16::from_le_bytes([a[0], a[1]]))
        .collect();
    
    for y in 0..8 {
        for x in 0..8 {
            print!("{:04X}",frame[FRAME_SX*y+x]);
        }
        println!();
    }

    Ok(())
}

fn readFrame(port: &Box<dyn SerialPort>)-> Result<Vec<i16>, Box<dyn std::error::Error>> {
    
}