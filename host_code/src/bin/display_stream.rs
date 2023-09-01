use std::{time::{Instant, Duration}, sync::mpsc::{self, SyncSender, Receiver}, thread, collections::VecDeque, io::Read};

use eframe::{egui, epaint::{Rect, Pos2, Rounding, Color32}};
use rand::Rng;

enum FrameMode {
    REALTIME,
    BURST_N(u64),
}
enum ExposureMode {
    SCALED,
    ABSOLUTE,
}

const NX: usize = 8;
const NY: usize = 8;
const RX_BUF_SIZE: usize = 64*1024;
const TARGET_FPS: f64 = 30.0; //CHECK Does the render hold things up and cause a pileup?
const FRAME_MODE: FrameMode = FrameMode::BURST_N(2*TARGET_FPS as u64);
const EXPOSURE_MODE: ExposureMode = ExposureMode::ABSOLUTE;


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

    let (tx_frame, rx_frame) = mpsc::sync_channel(1024*16);

    thread::spawn(move || {
        println!("Starting processor...");
        let mut header: VecDeque<u8> = VecDeque::new();
        let mut skips: usize = 0;
        loop {
            //DUMMY FR\n
            while header.len() < 3 {
                header.push_back(rx_byte.recv().expect("error receiving byte"));
            }

            if header[0] == b'F' && header[1] == b'R' && header[2] == b'\n' {
                let mut frame = vec![vec![0; NY]; NX];
                if skips > 0 {
                    println!("skipped {skips}");
                    skips = 0;
                }
                for y in 0..NY {
                    for x in 0..NX {
                        let lsb = rx_byte.recv().expect("error receiving byte");
                        let msb = rx_byte.recv().expect("error receiving byte");
                        frame[x][y] = (((msb as u16) << 8) | (lsb as u16)) as i16;
                    }
                }

                tx_frame.send(frame).expect("failed to send frame");
                header.clear();
            } else {
                skips = skips+1;
                header.push_back(rx_byte.recv().expect("error receiving byte"));
                header.pop_front();
            }
        }
    });

    let (tx_frame_capped, rx_frame_capped) = mpsc::sync_channel(1024);

    thread::spawn(move || {
        println!("Starting limiter...");
        let mut rx_tracker = TimedTracker::new(Duration::from_secs(10));
        let mut consecutive_skipped = 0;
        
        loop {
            match FRAME_MODE {
                FrameMode::REALTIME => {
                    let frame = rx_frame.recv().expect("failed to rx frame");

                    rx_tracker.add(());
                    let rx_fps = rx_tracker.countPerSecond();
                    let ratio = rx_fps / TARGET_FPS;
                    if consecutive_skipped as f64 >= ratio - 1.0 {
                        println!("processor stats {} {rx_fps} {TARGET_FPS} {ratio}", consecutive_skipped);
                        tx_frame_capped.send(frame).expect("failed to send frame (capped)");
                        consecutive_skipped = 0;
                    } else {
                        consecutive_skipped += 1;
                        continue;
                    }
                },
                FrameMode::BURST_N(burst_size) => {
                    let start = Instant::now();
                    let mut next_send = Instant::now();
                    let mut last_send = Instant::now();
                    let delay = Duration::from_secs_f64(1.0/TARGET_FPS);
                    // Send burst
                    for _ in 0..burst_size {
                        let frame = rx_frame.recv().expect("failed to rx frame");
                        println!("delay {}", last_send.elapsed().as_micros());
                        last_send = Instant::now();
                        tx_frame_capped.send(frame).expect("failed to send frame (capped)");
                        let n = Instant::now();
                        let nap = next_send.duration_since(n);
                        println!("napping {} {} {}", n.duration_since(start).as_micros(), next_send.duration_since(start).as_micros(), nap.as_micros());
                        thread::sleep(nap);
                        println!("napped {} {} ({})", nap.as_micros(), n.elapsed().as_micros(), delay.as_micros());
                        next_send = next_send.checked_add(delay).expect("time math failed");
                    }

                    // Skip built-up frames
                    let immediate = Duration::from_millis(0);
                    let mut _skipped = -1;
                    let mut done = false;
                    while !done {
                        _skipped += 1;
                        let res = rx_frame.recv_timeout(immediate);
                        done = match res {
                            Ok(_) => false,
                            Err(_) => true,
                        }
                    }
                    println!("skipped {_skipped} frames");
                },
            }
        }
    });

    let app = RenderApp {
        rx_frame: rx_frame_capped,
        last_time: Instant::now(),
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
    rx_frame: Receiver<Vec<Vec<i16>>>,
    last_time: Instant,
}

impl eframe::App for RenderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let cur_time = Instant::now();
            println!("elapsed ms out: {}", cur_time.duration_since(self.last_time).as_millis());
            self.last_time = cur_time;
            let mut rng = rand::thread_rng();

            let frame = self.rx_frame.recv().expect("failed to rx frame");

            let mut min: i16 = frame[0][0];
            let mut max: i16 = match EXPOSURE_MODE {
                ExposureMode::ABSOLUTE => 0x07FF,
                ExposureMode::SCALED => min,
            };

            for col in &frame {
                for v in col {
                    if v < &min {
                        min = *v;
                    } else if v > &max {
                        max = *v;
                    }
                }
            }

            let now = Instant::now();
            let p = ui.painter_at(Rect{min:Pos2{x:0 as f32, y:0 as f32}, max:Pos2{x:400.0,y:400.0}});
            for (y, col) in frame.iter().enumerate() {
                for (x, val) in col.iter().enumerate() {
                    // let n: u8 = *val as u8;
                    print!("{:04X} ", val);
                    let n = if max == min {
                        0xFF
                    } else {
                        (((val - min) as i32 * 255) / (max - min) as i32) as u8
                    };

                    p.rect_filled(Rect{min:Pos2{x:(x*10) as f32,y:(y*10) as f32}, max:Pos2{x:((x+1)*10) as f32,y:((y+1)*10) as f32}}, Rounding::none(), Color32::from_rgb(n, n, n));
                }
                println!();
            }
            let t: u128 = now.elapsed().as_micros();
            //println!("total {t}");

            let cur_time = Instant::now();
            println!("elapsed ms in:  {}", cur_time.duration_since(self.last_time).as_millis());
            self.last_time = cur_time;
        });
        ctx.request_repaint();
    }
}



//// TimedTracker
struct TimedTracker<T> {
    entries: VecDeque<(Instant, T)>,
    timeout: Duration,
}

impl<T> TimedTracker<T> {
    pub fn new(timeout: Duration) -> Self {
        Self {
            entries: VecDeque::new(),
            timeout: timeout,
        }
    }

    fn clean(&mut self) -> Instant {
        let t = Instant::now();
        let dead = t.checked_sub(self.timeout).expect("time math failed");
        self.entries.retain(|(t, _)| t > &dead);
        return t;
    }

    fn add(&mut self, v: T) {
        let t = self.clean();
        self.entries.push_back((t, v));
    }

    fn count(&mut self) -> usize {
        let t = self.clean();
        return self.entries.len();
    }

    fn countPerSecond(&mut self) -> f64 {
        let t = self.clean();
        return (self.entries.len() as f64) / self.timeout.as_secs_f64();
    }
}