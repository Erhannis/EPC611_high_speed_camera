use std::{time::{Instant, Duration}, sync::mpsc::{self, SyncSender, Receiver}, thread, collections::VecDeque, io::Read};

use eframe::{egui, epaint::{Rect, Pos2, Rounding, Color32}};
use rand::Rng;

const NX: usize = 8;
const NY: usize = 8;
const RX_BUF_SIZE: usize = 64*1024;


fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    // Create a new channel
    let (tx_byte, rx_byte) = mpsc::sync_channel(1024*1024);

    thread::spawn(move || {
        println!("Starting reader...");
        let device = ftdi::find_by_vid_pid(0x0403, 0x6014) // FT232H
            .interface(ftdi::Interface::A)
            .open();
            
        if let Ok(mut device) = device {
            println!("Device found and opened");
            device.usb_reset().unwrap();
            device.usb_purge_buffers().unwrap();
            device.set_latency_timer(16).unwrap();
    
            device.set_read_chunksize(0x10000);
            device.usb_set_event_char(None).unwrap();
            device.usb_set_error_char(None).unwrap();
            device.set_bitmode(0x00, ftdi::BitMode::Reset).unwrap();
    
            //device.write(&[1,2,3,4]).unwrap();
        
            loop {
                let mut buf: Vec<u8> = vec![0; RX_BUF_SIZE];
                device.read_exact(&mut buf).expect("Received no data!"); //RAINY Handle partial reads?

                for i in 0..buf.len() {
                    // Reverse bits, because the pico and ftdi are connected backwards
                    buf[i] = buf[i].reverse_bits();
                }

                for i in buf {
                    tx_byte.send(i).expect("Failed to send");
                }
            }
        } else {
            println!("Cannot find/open device, runtime tests are NOP");
        }
    });

    let (tx_frame, rx_frame) = mpsc::sync_channel(1024);

    thread::spawn(move || {
        println!("Starting processor...");
        loop {
            let mut frame = vec![vec![0; NY]; NX];
            for y in 0..NY {
                for x in 0..NX {
                    frame[x][y] = rx_byte.recv().expect("error receiving byte") as i16;
                    //DUMMY Frame sync, 2byte
                }
            }
            tx_frame.send(frame).expect("failed to send frame");
        }
    });

    let app = RenderApp {
        rx_frame: rx_frame
    };


    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Render",
        options,
        Box::new(|_cc| Box::<RenderApp>::from(app)),
    )
}

struct RenderApp {
    rx_frame: Receiver<Vec<Vec<i16>>>
}

impl eframe::App for RenderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut rng = rand::thread_rng();

            let frame = self.rx_frame.recv().expect("failed to rx frame");
            let now = Instant::now();
            let p = ui.painter_at(Rect{min:Pos2{x:0 as f32, y:0 as f32}, max:Pos2{x:400.0,y:400.0}});
            for (y, col) in frame.iter().enumerate() {
                for (x, val) in col.iter().enumerate() {
                    let n: u8 = *val as u8;
                    p.rect_filled(Rect{min:Pos2{x:(x*10) as f32,y:(y*10) as f32}, max:Pos2{x:((x+1)*10) as f32,y:((y+1)*10) as f32}}, Rounding::none(), Color32::from_rgb(n, n, n));
                }
            }
            let t: u128 = now.elapsed().as_micros();
            //println!("total {t}");
        });
        ctx.request_repaint();
    }
}
