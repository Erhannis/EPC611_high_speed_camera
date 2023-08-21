extern crate serialport;

use std::cmp::max;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use serialport::SerialPort;

const FRAME_SX: usize = 8;
const FRAME_SY: usize = 8;
const FRAME_BYTES: usize = FRAME_SX*FRAME_SY*2;

//DUMMY Do error handling better

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new channel
    let (tx, rx) = mpsc::sync_channel(1024*1024);

    // Spawn a new thread and move the sending end into it
    thread::spawn(move || {
        // let mut port = serialport::new("/dev/ttyUSB0", 115_200)

        let mut port = serialport::new("/dev/ttyUSB0", 921600)
        
        // let mut port = serialport::new("/dev/ttyUSB0", 1_000_000)
        //let mut port = serialport::new("/dev/ttyUSB0", 1_500_000)
            .timeout(Duration::from_millis(3000))
            .open().expect("Failed to open port");
        
        loop {
            let mut serial_buf: Vec<u8> = vec![0; 128+3];
            port.read_exact(serial_buf.as_mut_slice()).expect("Found no data!");
 
            for i in serial_buf {
                tx.send(i).unwrap();
            }
        }
    });

    let mut times: VecDeque<u128> = VecDeque::new();
    for _ in 0..100 {
        times.push_back(0);
    }

    // let output = "This is a test. This is only a test.".as_bytes();
    // port.write(output).expect("Write failed!");

    let now0 = Instant::now();
    let mut timer: u128 = 0;

    loop {
        
        // let t: u128 = micros();
        // let diff: u128 = t - timer;
        // Serial.printf("out: %ld", diff);
        // timer = t;
        
        let frame: Vec<i16> = readFrame(&rx).expect("failed to read frame");

        let t: u128 = now0.elapsed().as_micros();
        let diff = t - timer;
        timer = t;
        times.push_back(diff);
        times.pop_front();
        let avg: u128 = times.iter().sum::<u128>() / times.len() as u128;

        // for y in 0..8 {
        //     for x in 0..8 {
        //         print!("{:04X} ", frame[FRAME_SX*y+x]);
        //     }
        //     println!();
        // }
        // println!();

        printFrame(&frame, FRAME_SX, FRAME_SY)?;

        println!();
        println!("    {}us  ===", diff);
        println!("avg {} ===", avg);
        println!();
    }

    Ok(())
}

fn readFrame(rx: &Receiver<u8>)-> Result<Vec<i16>, Box<dyn std::error::Error>> {
    let mut lookahead: VecDeque<u8> = VecDeque::new();

    lookahead.push_back(rx.recv()?);
    lookahead.push_back(rx.recv()?);
    lookahead.push_back(rx.recv()?);

    while !(lookahead[0] == b'F' && lookahead[1] == b'R' && lookahead[2] == b'\n') {
        lookahead.pop_front();
        lookahead.push_back(rx.recv()?);
    }
    
    let mut frameBytes: Vec<u8> = vec![0; FRAME_BYTES];
    for i in 0..FRAME_BYTES {
        frameBytes[i] = rx.recv()?;
    }

    //LEAK This could be slow - though probably not enough to matter.
    let frame: Vec<i16> = frameBytes
        .chunks_exact(2)
        .into_iter()
        .map(|a| i16::from_le_bytes([a[0], a[1]]))
        .collect();

    return Ok(frame);
}

fn printFrame(frame: &Vec<i16>, nx: usize, ny: usize) -> Result<(), Box<dyn std::error::Error>> {
    // Define the characters for shading
    let shades = [' ', '░', '▒', '▓', '█'];

    let fmax = *(frame.iter().max().ok_or("Failed to find max in frame")?);
    let fmax = max(1, fmax);

    println!();
    for _ in 0..fmax.ilog2() {
        print!("X");
    }
    println!();
    println!();

    let div = 1; //RAINY Implement, maybe
    // Iterate over the pixels
    for y in 0..ny {
        if y % div != 0 {
            continue;
        }
        for x in 0..nx {
            if x % div != 0 {
                continue;
            }

            let level1 = max(frame[nx*y+x], 0) as usize;
            let level2 = level1 * (shades.len() - 1);
            let level3 = level2 / (fmax as usize);
            let level = level3;

            // Print the corresponding character
            print!("{}", shades[level]);
            print!("{}", shades[level]);
        }
        println!();
    }

    return Ok(());
}
