use std::error::Error;

use image::imageops::rotate90;
use image::DynamicImage;
use image::DynamicImage::ImageRgba8;
use image::Rgba;
use image::GenericImage;
use image::GenericImageView;

#[cfg(not(windows))]
macro_rules! os_separator{
    ()=>{"/"}
}

#[cfg(windows)]
macro_rules! os_separator{
    ()=>{r#"\"#}
}

// TODO: handle errors better, test on linux
pub fn overlay_bi_flag(img: DynamicImage) -> DynamicImage {
    let mut new_img = img.clone();
    let bi_bytes = include_bytes!(concat!(
        "..",
        os_separator!(),
        "res",
        os_separator!(),
        "bi.png"
    ));
    let mut bi_image = match image::load_from_memory(bi_bytes){
        Ok(img) => img,
        Err(_e) => {
            // Just return original image if there is an error for now
            return img;
        }
    };

    bi_image = bi_image.resize_exact(img.width(), img.height(), image::imageops::Nearest);
    for x in 0..img.width() {
        for y in 0..img.height() {
            new_img.put_pixel(x, y, average_pixel(
                img.get_pixel(x, y),
                bi_image.get_pixel(x, y)
            ));
        }
    }
    new_img
}

pub fn rotate_image(img: DynamicImage) -> DynamicImage {
    ImageRgba8(rotate90(&img))
}

fn average_pixel(block: Rgba<u8>, input: Rgba<u8>) -> Rgba<u8> {
    image::Rgba([
        (block[0] / 2) + (input[0] / 2),
        (block[1] / 2) + (input[1] / 2),
        (block[2] / 2) + (input[2] / 2),
        block[3] // Keeps transparent pixels
    ])
}

pub fn vec_image(img: &DynamicImage) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut vec: Vec<u8> = Vec::new();
    if let Err(e) = img.write_to(&mut vec, image::ImageOutputFormat::Png) {
        return Err(Box::new(e))
    };
    
    Ok(vec)
}