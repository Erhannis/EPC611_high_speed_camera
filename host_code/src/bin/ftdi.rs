//! A simple test script for testing the loopback features
//! of the FTDI DLP-HS-FPGA3's default FPGA firmware.
//! Mainly used as a compile test

extern crate ftdi;

use std::{io::{Read, Write}, time::Instant, convert::TryInto, convert::TryFrom};
use std::cmp::min;

const RX_BUF_SIZE: usize = 0x100000;//*0x200 //CHECK Try large values, now?
// const MAX_PRINT_SIZE: usize = 0x100;
const MAX_PRINT_SIZE: usize = 0x10000000;
const ITER: i32 = 0x01;

fn main() {
    println!("Starting tester...");
    let device = ftdi::find_by_vid_pid(0x0403, 0x6014) // FT232H
        .interface(ftdi::Interface::A)
        .open();

    if let Ok(mut device) = device {
        println!("Device found and opened");
        device.usb_reset().unwrap();
        device.usb_purge_buffers().unwrap();
        device.set_latency_timer(16).unwrap();

        // Missing: set_usb_parameters
        device.set_read_chunksize(0x10000);
        device.usb_set_event_char(None).unwrap();
        device.usb_set_error_char(None).unwrap();
        // Missing: set_timeouts
        //ft.set_latency_timer(Duration::from_millis(16))?;
        //ft.set_flow_control_rts_cts()?;
        device.set_bitmode(0x00, ftdi::BitMode::Reset).unwrap();

        // Ok, so - setting this or not setting this gives different behavior, which is weird, because I don't think you can set CPU FIFO mode from code?
        // AUGH.  Removing this gives good behavior...?
        //device.set_bitmode(0x00, ftdi::BitMode::Syncff).unwrap(); // Synchronous FIFO

        //device.write(&[1,2,3,4]).unwrap();

        let mut buf0: Vec<u8> = vec![0; RX_BUF_SIZE];
        let mut buf1: Vec<u8> = vec![0; RX_BUF_SIZE];

        let now0 = Instant::now();
        let mut total: u128 = 0;
        for _ in 0..ITER {
            // print!(". ");
            let now = Instant::now();
            device.read_exact(&mut buf0).unwrap(); //DUMMY Handle partial reads
            let t: u128 = now.elapsed().as_micros();
            let z: u128 = (RX_BUF_SIZE * 1000000).try_into().unwrap();
            total += u128::try_from(RX_BUF_SIZE).unwrap();

            for i in 0..buf0.len() {
                buf0[i] = buf0[i].reverse_bits();
            }
                
            let mut last: u8 = 0b00000000; //DUMMY May skip first byte, or erroneously admit it
            let mut j = 0;
            if false {
                for i in 0..buf0.len() {
                    if last != (buf0[i] & 0b10000000) {
                        buf1[j] = buf0[i] & 0b01111111;
                        j += 1;
                    }
                    last = buf0[i] & 0b10000000;
                }
            }

            if false { // 019,019,148,148,021,021
                let n0 = min(buf0.len(), MAX_PRINT_SIZE);
                print!("rx0: ");
                if n0 < buf0.len() {
                    for i in 0..n0 {
                        print!("{:#03},", buf0[i]);
                    }
                    print!("...");
                    for i in (buf0.len()-n0)..buf0.len() {
                        print!("{:#03},", buf0[i]);
                    }
                } else {
                    for i in 0..n0 {
                        print!("{:#03},", buf0[i]);
                    }
                }
                println!();                
            }

            if false { // 015, 015,-016,-016, 017, 017
                let n0 = min(buf0.len(), MAX_PRINT_SIZE);
                last = 0;
                print!("rx0: ");
                if n0 < buf0.len() {
                    for i in 0..n0 {
                        let mut x = buf0[i];
                        if x & 0b10000000 != 0 {
                            print!("-{:#03},", x&0b01111111);
                        } else {
                            print!(" {:#03},", x&0b01111111);
                        }
                    }
                    print!("...");
                    for i in (buf0.len()-n0)..buf0.len() {
                        let mut x = buf0[i];
                        if x & 0b10000000 != 0 {
                            print!("-{:#03},", x&0b01111111);
                        } else {
                            print!(" {:#03},", x&0b01111111);
                        }
                    }
                } else {
                    for i in 0..n0 {
                        let s = buf0[i]&0b10000000;
                        let x = buf0[i]&0b01111111;
                        let c: char = if s != 0 { '-' } else { ' ' };
                        if x == ((last+1) % 128) || x == last {
                            print!("{}{:#03},", c, x&0b01111111);
                        } else {
                            print!("\x1b[31m{}{:#03}\x1b[0m,", c, x&0b01111111);
                        }
                        last = x;
                    }
                }
                println!();                
            }
            
            if false { // 069,070,071,072
                // let n = min(buf0.len(), MAX_PRINT_SIZE);
                let n1 = min(j, MAX_PRINT_SIZE);
                print!("rx1: ");
                last = 0;
                if n1 < j {
                    for i in 0..n1 {
                        if buf1[i] == ((last+1) % 128) {
                            print!("{:#03},", buf1[i]);
                        } else {
                            print!("\x1b[31m{:#03}\x1b[0m,", buf1[i]);
                        }
                        last = buf1[i];
                    }
                    print!("...");
                    for i in (j-n1)..j {
                        if buf1[i] == ((last+1) % 128) {
                            print!("{:#03},", buf1[i]);
                        } else {
                            print!("\x1b[31m{:#03}\x1b[0m,", buf1[i]);
                        }
                        last = buf1[i];
                    }
                } else {
                    let mut skips: u32 = 0;
                    let mut skiplens: u64 = 0;
                    let mut skipTotal: u64 = 0;
                    let mut curSkip: u64 = 0;
                    for i in 0..n1 {
                        if buf1[i] == ((last+1) % 128) {
                            print!("{:#03},", buf1[i]);
                            curSkip += 1;
                        } else {
                            print!("\x1b[31m{:#03}\x1b[0m,", buf1[i]);
                            skips += 1;
                            skiplens += curSkip;
                            if (buf1[i] < last) {
                                skipTotal += (buf1[i] as u64) + 128 - (last as u64);
                            } else {
                                skipTotal += (buf1[i] - last) as u64;
                            }
                            curSkip = 0;
                        }
                        last = buf1[i];
                    }
                    println!();
                    println!();
                    println!("skips {skips} missed avg {:.2} entries, bookending {:.2} entries", (skipTotal as f64) / (skips as f64), (skiplens as f64) / (skips as f64));
                }
            }

            if true { // 125,126,127,128,129
                let n = min(buf0.len(), MAX_PRINT_SIZE);
                print!("rx1: ");
                last = 0;
                if n < buf0.len() {
                    for i in 0..n {
                        if buf0[i] == ((last.overflowing_add(1).0)) {
                            print!("{:#03},", buf0[i]);
                        } else {
                            print!("\x1b[31m{:#03}\x1b[0m,", buf0[i]);
                        }
                        last = buf0[i];
                    }
                    print!("...");
                    for i in (buf0.len()-n)..buf0.len() {
                        if buf0[i] == ((last.overflowing_add(1).0)) {
                            print!("{:#03},", buf0[i]);
                        } else {
                            print!("\x1b[31m{:#03}\x1b[0m,", buf0[i]);
                        }
                        last = buf0[i];
                    }
                } else {
                    let mut skipList: Vec<usize> = Vec::new();
                    let mut skips: u32 = 0;
                    let mut skiplens: u64 = 0;
                    let mut skipTotal: u64 = 0;
                    let mut curSkip: u64 = 0;
                    for i in 0..n {
                        if buf0[i] == ((last.overflowing_add(1).0)) {
                            print!("{:#03},", buf0[i]);
                            curSkip += 1;
                        } else {
                            print!("\x1b[31m{:#03}\x1b[0m,", buf0[i]);
                            skipList.push(i);
                            skips += 1;
                            skiplens += curSkip;
                            skipTotal += buf0[i].wrapping_sub(last) as u64;
                            curSkip = 0;
                        }
                        last = buf0[i];
                    }
                    println!();
                    println!();
                    println!("skips {skips} missed avg {:.2} entries, bookending {:.2} entries", (skipTotal as f64) / (skips as f64), (skiplens as f64) / (skips as f64));
                    println!("giving error rate 1/{:.6}", (buf0.len() as f64) / (skipTotal as f64)); //DUMMY This is strange in some cases, like flatline, technically skips nothing
                    if skipList.len() <= 10 {
                        println!("Skips: {:?}", skipList);
                    }
                }
            }


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

            // if n < display_buf.len() {
            //     for i in 0..n {
            //         print!("{:#018b}, {:#03},\n", rx_buf[i], rx_buf[i]);
            //         print!("{:#018b}, {:#03},\n\n", display_buf[i], display_buf[i]);
            //     }
            //     print!("...");
            //     for i in (display_buf.len()-n)..display_buf.len() {
            //         print!("{:#018b}, {:#03},\n", rx_buf[i], rx_buf[i]);
            //         print!("{:#018b}, {:#03},\n\n", display_buf[i], display_buf[i]);
            //     }
            // } else {
            //     for i in 0..n {
            //         print!("{:#018b}, {:#03},\n", rx_buf[i], rx_buf[i]);
            //         print!("{:#018b}, {:#03},\n\n", display_buf[i], display_buf[i]);
            //     }
            // }

            println!();
            println!();

            println!("{RX_BUF_SIZE} @ {} = {} B/s", t, z/t);
            println!();
            println!("{RX_BUF_SIZE} : {j} = {:.2}", (j as f64) / (RX_BUF_SIZE as f64));

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
