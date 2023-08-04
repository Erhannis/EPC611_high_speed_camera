//! A simple test script for testing the loopback features
//! of the FTDI DLP-HS-FPGA3's default FPGA firmware.
//! Mainly used as a compile test

extern crate ftdi;

use std::{io::{Read, Write}, time::Instant, convert::TryInto, convert::TryFrom};
use std::cmp::min;

const RX_BUF_SIZE: usize = 0x10000*3;//*0x200 //CHECK Try large values, now?
const MAX_PRINT_SIZE: usize = 0x10;
// const MAX_PRINT_SIZE: usize = 0x1000000;
const ITER: i32 = 0x10;

fn main() {
    println!("Starting tester...");
    let device = ftdi::find_by_vid_pid(0x0403, 0x6014) // FT232H
        .interface(ftdi::Interface::A)
        .open();

    if let Ok(mut device) = device {
        println!("Device found and opened");
        device.usb_reset().unwrap();
        device.usb_purge_buffers().unwrap();
        device.set_latency_timer(2).unwrap();

        // Missing: set_usb_parameters
        device.usb_set_event_char(None).unwrap();
        device.usb_set_error_char(None).unwrap();
        // Missing: set_timeouts
        //ft.set_latency_timer(Duration::from_millis(16))?;
        //ft.set_flow_control_rts_cts()?;
        device.set_bitmode(0x00, ftdi::BitMode::Reset).unwrap();
        device.set_bitmode(0x00, ftdi::BitMode::Syncff).unwrap(); // Synchronous FIFO

        let mut rx_buf: Vec<u8> = vec![0; RX_BUF_SIZE];
        let mut display_buf: Vec<i16> = vec![0; RX_BUF_SIZE];

        let now0 = Instant::now();
        let mut total: u128 = 0;
        for _ in 0..ITER {
            // print!(". ");
            let now = Instant::now();
            device.read_exact(&mut rx_buf).unwrap();
            let t: u128 = now.elapsed().as_micros();
            let z: u128 = (RX_BUF_SIZE * 1000000).try_into().unwrap();
            total += u128::try_from(RX_BUF_SIZE).unwrap();
    
            println!("{RX_BUF_SIZE} @ {} = {} B/s", t, z/t);

            let n = min(rx_buf.len(), MAX_PRINT_SIZE);

            for i in 0..rx_buf.len() {
                if rx_buf[i] & 0b10000000 != 0 {
                    display_buf[i] = -((rx_buf[i] & 0b01111111) as i16);
                } else {
                    display_buf[i] =   (rx_buf[i] & 0b01111111) as i16;
                }

                // display_buf[i] = rx_buf[i] as i16;

                // display_buf[i] =   (rx_buf[i] & 0b01111111) as i16;

                // display_buf[i] = rx_buf[i] as i8 as i16;
            }

            print!("rx: ");

            // if n < display_buf.len() {
            //     for i in 0..n {
            //         print!("{:#03},", display_buf[i]);
            //     }
            //     print!("...");
            //     for i in (rx_buf.len()-n)..display_buf.len() {
            //         print!("{:#03},", display_buf[i]);
            //     }
            // } else {
            //     for i in 0..n {
            //         print!("{:#03},", display_buf[i]);
            //     }
            // }

            // if n < rx_buf.len() {
            //     for i in 0..n {
            //         print!("{:#03},", rx_buf[i]);
            //     }
            //     print!("...");
            //     for i in (rx_buf.len()-n)..rx_buf.len() {
            //         print!("{:#03},", rx_buf[i]);
            //     }
            // } else {
            //     for i in 0..n {
            //         print!("{:#03},", rx_buf[i]);
            //     }
            // }

            // if n < rx_buf.len() {
            //     for i in 0..n {
            //         print!("{:#010b},\n", rx_buf[i]);
            //     }
            //     print!("...");
            //     for i in (rx_buf.len()-n)..rx_buf.len() {
            //         print!("{:#010b},\n", rx_buf[i]);
            //     }
            // } else {
            //     for i in 0..n {
            //         print!("{:#010b},\n", rx_buf[i]);
            //     }
            // }

            if n < display_buf.len() {
                for i in 0..n {
                    print!("{:#018b}, {:#03},\n", rx_buf[i], rx_buf[i]);
                    print!("{:#018b}, {:#03},\n\n", display_buf[i], display_buf[i]);
                }
                print!("...");
                for i in (display_buf.len()-n)..display_buf.len() {
                    print!("{:#018b}, {:#03},\n", rx_buf[i], rx_buf[i]);
                    print!("{:#018b}, {:#03},\n\n", display_buf[i], display_buf[i]);
                }
            } else {
                for i in 0..n {
                    print!("{:#018b}, {:#03},\n", rx_buf[i], rx_buf[i]);
                    print!("{:#018b}, {:#03},\n\n", display_buf[i], display_buf[i]);
                }
            }

            println!();
            println!();
            //ft.write_all(&rx_buf)?;
        }
    
        let t: u128 = now0.elapsed().as_micros();
        let z: u128 = (total * 1000000).try_into().unwrap();
        println!("total {total} @ {} = {} B/s", t, z/t);
    
        println!();
    
        // Missing: close





        // // Junk test
        // let mut junk = vec![];
        // device.read_to_end(&mut junk).unwrap();
        // if junk.len() > 0 {
        //     println!("Junk in line: {:?}", junk);
        // }

        // // Ping test
        // device.write_all(&vec![0x00]).unwrap();
        // let mut reply = vec![];
        // device.read_to_end(&mut reply).unwrap();
        // if reply != vec![0x56] {
        //     println!("Wrong ping reply {:?} (expected {:?}", reply, vec![0x56]);
        // }

        // for num in 0u16..256 {
        //     let num = num as u8;

        //     // Loopback test
        //     device.write_all(&vec![0x20, num]).unwrap();
        //     let mut reply = vec![];
        //     device.read_to_end(&mut reply).unwrap();
        //     if reply != vec![num] {
        //         println!("Wrong loopback reply {:?} (expected {:?}", reply, vec![num]);
        //     }

        //     // Complement loopback test
        //     device.write_all(&vec![0x21, num]).unwrap();
        //     let mut reply = vec![];
        //     device.read_to_end(&mut reply).unwrap();
        //     let complement = 255 - num;
        //     if reply != vec![complement] {
        //         println!(
        //             "Wrong complement reply {:?} (expected {:?}",
        //             reply,
        //             vec![complement]
        //         );
        //     }
        // }
        println!("Testing finished");
    } else {
        println!("Cannot find/open device, runtime tests are NOP");
    }
}
