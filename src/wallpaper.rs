use image::io::Reader as ImageReader;
use kmeans_colors::{get_kmeans_hamerly, Kmeans, Sort};
use palette::{
    cast::from_component_slice,
    convert::{FromColor, IntoColor},
    Lab, LinSrgb, LinSrgba,
};
use rand::Rng;
use std::{error::Error, path::PathBuf, process::Command};

use urlencoding;

pub type ColorPercentPair = (LinSrgb<u8>, f32);

pub fn get_dominant_colors(max_colors: usize) -> Result<Vec<ColorPercentPair>, Box<dyn Error>> {
    let wallpaper_path_str = get_wallpaper_path()?;
    let lab_pixels = load_image_as_lab(&PathBuf::from(&wallpaper_path_str))?;

    let max_runs = 10;
    let result = find_kmeans(&lab_pixels, max_runs, max_colors);

    // Convert to sRGB
    let rgb_percent = Lab::sort_indexed_colors(&result.centroids, &result.indices)
        .iter()
        .map(|x| (LinSrgb::from_color(x.centroid).into_format(), x.percentage))
        .collect::<Vec<(LinSrgb<u8>, f32)>>();

    Ok(rgb_percent)
}

fn get_wallpaper_path() -> Result<String, Box<dyn Error>> {
    // Consider using zbus to get this value rather than spawning another process
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.background", "picture-uri"])
        .output()?;
    let path = String::from_utf8(output.stdout)?
        .trim()
        .trim_matches('\'')
        .trim_start_matches("file://")
        .to_string();
    Ok(urlencoding::decode(&path)?.into_owned())
}

fn load_image_as_lab(path: &PathBuf) -> Result<Vec<Lab>, Box<dyn Error>> {
    // The author of kmeans_color says that using Lab will produce more perceptually accurate
    // results, while RGB will converge faster, convert to Lab for processing.
    let img = ImageReader::open(path)?.decode()?;
    let resized = img.thumbnail(800, 800);
    let raw_pixels: Vec<u8> = resized.into_rgba8().into_raw();

    let labs: Vec<Lab> = from_component_slice::<LinSrgba<u8>>(&raw_pixels)
        .iter()
        .map(|px_srgba| px_srgba.into_format::<f32, f32>().into_color())
        .collect();

    Ok(labs)
}

fn find_kmeans(lab_pixels: &[Lab], max_runs: usize, max_colors: usize) -> Kmeans<Lab> {
    let max_iterations = 20;
    let converge = 5.0;
    let verbose = false;

    let mut result = Kmeans::new();

    for _i in 0..max_runs {
        let run_result = get_kmeans_hamerly(
            max_colors,
            max_iterations,
            converge,
            verbose,
            lab_pixels,
            rand::thread_rng().gen(),
        );
        if run_result.score < result.score {
            result = run_result;
        }
    }
    result
}
