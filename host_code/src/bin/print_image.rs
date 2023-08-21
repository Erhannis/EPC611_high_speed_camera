extern crate image;

use image::GenericImageView;
use crate::image::Pixel;

fn main() {
    // Load the image
    let img = image::open("/home/erhannis/Downloads/dbvptl0-978fb6f7-f721-4134-8abf-9c3780e2729a.png").unwrap();
    let (width, height) = img.dimensions();

    // Define the characters for shading
    let shades = [' ', '░', '▒', '▓', '█'];

    let div = 50;
    // Iterate over the pixels
    for y in 0..height {
        if y % div != 0 {
            continue;
        }
        for x in 0..width {
            if x % div != 0 {
                continue;
            }

                // Get the pixel value
            let pixel = img.get_pixel(x, y).to_luma();
            let level = pixel[0] as usize * (shades.len() - 1) / 255;

            // Print the corresponding character
            print!("{}", shades[level]);
            print!("{}", shades[level]);
        }
        println!();
    }
}