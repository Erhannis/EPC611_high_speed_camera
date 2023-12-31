use std::{time::{Instant, Duration}, sync::{mpsc::{self, SyncSender, Receiver}, RwLock, Arc}, thread, collections::VecDeque, io::Read, f64::consts::PI};

use eframe::{egui, epaint::{Rect, Pos2, Rounding, Color32}};
use portable_atomic::AtomicF64;
use rand::Rng;

enum FrameMode {
    REALTIME,
    BURST_N(u64),
}
enum ExposureMode {
    SCALED,
    ABSOLUTE,
}
enum FramePrintMode {
    HEX,
    DOT,
    NONE,
}

const NX: usize = 8;
const NY: usize = 8;
const RX_BUF_SIZE: usize = 64*1024;
// const RX_BUF_SIZE: usize = 1*1024;
const TARGET_FPS: f64 = 30.0; //CHECK Does the render hold things up and cause a pileup?
const TARGET_LATENCY: f64 = 0.5;
const CATCHUP_FACTOR: f64 = 2.0;

const DUMMY_MODE: bool = false;

const FRAME_MODE: FrameMode = FrameMode::BURST_N(2*TARGET_FPS as u64);
// const FRAME_MODE: FrameMode = FrameMode::REALTIME;

const EXPOSURE_MODE: ExposureMode = ExposureMode::ABSOLUTE;
// const EXPOSURE_MODE: ExposureMode = ExposureMode::SCALED;

const FRAME_PRINT_MODE: FramePrintMode = FramePrintMode::DOT;
// const FRAME_PRINT_MODE: FramePrintMode = FramePrintMode::HEX;

const IMMEDIATE: Duration = Duration::from_millis(0);


fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    // Create a new channel
    let (tx_byte, rx_byte) = mpsc::sync_channel(1024*1024);

    if DUMMY_MODE {
        thread::spawn(move || {
            println!("Starting dummy reader...");
            let mut rng = rand::thread_rng();
            let mut i: u64 = 0;
            let mut rate = RateLimiter::new(Duration::from_millis(2));
            loop {
                tx_byte.send(b'F').expect("Failed to send");
                tx_byte.send(b'R').expect("Failed to send");
                tx_byte.send(b'\n').expect("Failed to send");

                for _ in 0..NY {
                    for _ in 0..NX {
                        // let val: i16 = rng.gen_range(-0x10..=0x07FF);
                        let val: i16 = ((0x07FF as f64)*((PI+(i as f64)/500.0).sin()+1.0)/2.0) as i16;
                        // println!("{}", val);
                        tx_byte.send((val & 0xFF) as u8).expect("Failed to send");
                        tx_byte.send((val >> 8) as u8).expect("Failed to send");
                    }
                }
                rate.interval_wait();
                i += 1;
            }
        });
    } else {
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
                    // let count = device.read(&mut buf).expect("Received no data!");
                    device.read_exact(&mut buf).expect("Received no data!"); //RAINY Handle partial reads?
                    let count = buf.len();

                    for i in 0..count {
                        // Reverse bits, because the pico and ftdi are connected backwards
                        let v = buf[i].reverse_bits();
                        tx_byte.send(v).expect("Failed to send");
                    }
                }
            } else {
                println!("Cannot find/open device, runtime tests are NOP");
            }
        });
    }

    let (tx_frame, rx_frame) = mpsc::sync_channel(1024*16);
    let rx_fps0 = Arc::new(AtomicF64::new(0.0));

    let rx_fps = rx_fps0.clone();
    thread::spawn(move || {
        println!("Starting bytes-to-frames processor...");
        let mut header: VecDeque<u8> = VecDeque::new();
        let mut skips: usize = 0;
        let mut tx_tracker = TimedTracker::new(Duration::from_secs(10));
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

                // tx_frame.send(frame).expect("failed to send frame");
                tx_frame.send(frame).unwrap_or(()); //DUMMY
                
                tx_tracker.add(());
                rx_fps.store(tx_tracker.countPerSecond(), portable_atomic::Ordering::Relaxed);
                header.clear();
            } else {
                skips = skips+1;
                header.push_back(rx_byte.recv().expect("error receiving byte"));
                header.pop_front();
            }
        }
    });

    let (tx_frame_capped, rx_frame_capped) = mpsc::sync_channel(1024);

    let rx_fps = rx_fps0.clone();
    thread::spawn(move || {
        println!("Starting limiter...");
        let mut buffer: VecDeque<Vec<Vec<i16>>> = VecDeque::new();

        let mut fm_next_send = Instant::now();
        let mut fm_last_send = Instant::now();
        let frame_delay = Duration::from_secs_f64(1.0/TARGET_FPS);
        let mut fps_timer = RateLimiter::new(frame_delay);
    
        loop {
            //SHAME These are specific to FRAME_MODE

            match FRAME_MODE {
                FrameMode::REALTIME => {
                    // Pull available frames into buffer
                    // let mut _skipped = -1;
                    let mut done = false;
                    while !done {
                        // _skipped += 1;
                        let res = rx_frame.recv_timeout(IMMEDIATE);
                        done = match res {
                            Ok(_) => {
                                buffer.push_back(res.unwrap());
                                false // Hot tip: you can't do `return blah` here, it'll return your surrounding function.  :|
                            },
                            Err(_) => true,
                        };
                    }
                    // println!("skipped {_skipped} frames");

                    // So we have all available frames, and probably some extra.
                    // We also have the average rx_fps.
                    // And a target fps.
                    // So we skip N-1 frames, where 1/N = target_fps/rx_fps .
                    //DUMMY //NEXT Handle gradual accumulations.

                    let rx_fps = rx_fps.load(portable_atomic::Ordering::Relaxed);
                    let f = if buffer.len() as f64 > TARGET_LATENCY * rx_fps {
                        CATCHUP_FACTOR
                    } else {
                        1.0
                    };
                    let n = (f * rx_fps / TARGET_FPS) as usize;

                    if n > 1 {
                        for _ in 0..(n-1) {
                            buffer.pop_front();
                        }
                    }

                    // And show the next frame.
                    println!("frame buffer: {}", buffer.len());
                    println!("base fps {}", rx_fps);
                    if let Some(frame) = buffer.pop_front() {
                        tx_frame_capped.send(frame).expect("failed to send frame (capped)");
                    } else {
                        println!("Frame buffer underrun");
                    }

                    // Then delay for target fps.
                    fps_timer.interval_wait(); //THINK Under certain circumstance, delay_wait could be better.
                },
                FrameMode::BURST_N(burst_size) => {
                    let start = Instant::now();
                    let mut next_send = Instant::now();
                    let mut last_send = Instant::now();
                    // Send burst
                    for _ in 0..burst_size {
                        let frame = rx_frame.recv().expect("failed to rx frame");
                        println!("delay {}", last_send.elapsed().as_micros());
                        last_send = Instant::now();
                        let rx_fps = rx_fps.load(portable_atomic::Ordering::Relaxed);
                        println!("base fps {}", rx_fps);
                        tx_frame_capped.send(frame).expect("failed to send frame (capped)");
                        let n = Instant::now();
                        let nap = next_send.duration_since(n);
                        println!("napping {} {} {}", n.duration_since(start).as_micros(), next_send.duration_since(start).as_micros(), nap.as_micros());
                        thread::sleep(nap);
                        println!("napped {} {} ({})", nap.as_micros(), n.elapsed().as_micros(), frame_delay.as_micros());
                        next_send = next_send.checked_add(frame_delay).expect("time math failed");
                    }

                    // Skip built-up frames
                    let mut _skipped = -1;
                    let mut done = false;
                    while !done {
                        _skipped += 1;
                        let res = rx_frame.recv_timeout(IMMEDIATE);
                        done = match res {
                            Ok(_) => false,
                            Err(_) => true,
                        };
                    }
                    println!("skipped {_skipped} frames");
                },
            }
        }
    });

    let app = RenderApp {
        rx_frame: rx_frame_capped,
        last_time: Instant::now(),
        rx_fps: rx_fps0.clone(),
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
    rx_fps: Arc<AtomicF64>,
}

impl eframe::App for RenderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let cur_time = Instant::now();
            println!("elapsed ms out: {}", cur_time.duration_since(self.last_time).as_millis());
            self.last_time = cur_time;
            let mut rng = rand::thread_rng();

            // let frame = self.rx_frame.recv().expect("failed to rx frame");
            let frame = self.rx_frame.recv();
            if let Err(e) = frame {
                println!("Error {}", e);
                return;
            }
            let frame = frame.unwrap();

            let mut min: i16 = match EXPOSURE_MODE {
                ExposureMode::ABSOLUTE => 0,
                ExposureMode::SCALED => frame[0][0],
            };
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

            let PIX_SX = 10;
            let PIX_SY = 10;

            let now = Instant::now();
            let p = ui.painter_at(Rect{min:Pos2{x:0 as f32, y:0 as f32}, max:Pos2{x:400.0,y:400.0}});
            for (y, col) in frame.iter().enumerate() {
                for (x, val) in col.iter().enumerate() {
                    // let n: u8 = *val as u8;
                    if matches!(FRAME_PRINT_MODE, FramePrintMode::HEX) {
                        print!("{:04X} ", val);
                    }
                    let n = if max == min {
                        0xFF
                    } else {
                        ((((*val as i32) - (min as i32)) * 255) / ((max as i32) - (min as i32))) as u8
                    };

                    p.rect_filled(Rect{min:Pos2{x:(x*PIX_SX) as f32,y:(y*PIX_SY) as f32}, max:Pos2{x:((x+1)*PIX_SX) as f32,y:((y+1)*PIX_SY) as f32}}, Rounding::none(), Color32::from_rgb(n, n, n));
                }
                if matches!(FRAME_PRINT_MODE, FramePrintMode::HEX) {
                    println!();
                }
            }
            let t: u128 = now.elapsed().as_micros();
            // println!("total {t}");
            if matches!(FRAME_PRINT_MODE, FramePrintMode::DOT) {
                println!(".");
            }



            // Other UI
            ui.horizontal(|ui| {
                ui.add_space((frame[0].len()*PIX_SX) as f32);
                ui.vertical(|ui| {
                    ui.label(format!("Base FPS: {}", self.rx_fps.load(portable_atomic::Ordering::Relaxed)));
                    ui.label(format!("Base FPS: {}", self.rx_fps.load(portable_atomic::Ordering::Relaxed)));
                });
            });




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

struct RateLimiter {
    next_time: Instant,
    timeout: Duration,
}

impl RateLimiter {
    pub fn new(timeout: Duration) -> Self {
        Self {
            next_time: Instant::now(),
            timeout: timeout,
        }
    }

    fn go(&mut self) -> bool {
        let n = Instant::now();
        if n >= self.next_time {
            self.next_time = n.checked_add(self.timeout).expect("time math failed");
            return true;
        }
        return false;
    }
    
    fn interval_wait(&mut self) {
        let delay = self.next_time.duration_since(Instant::now());
        if !delay.is_zero() {
            thread::sleep(delay);
        }
        self.next_time = self.next_time.checked_add(self.timeout).expect("time math failed");
    }

    fn delay_wait(&mut self) {
        let delay = self.next_time.duration_since(Instant::now());
        if !delay.is_zero() {
            thread::sleep(delay);
        }
        self.next_time = Instant::now().checked_add(self.timeout).expect("time math failed");
    }
}