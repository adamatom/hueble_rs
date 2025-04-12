use clap::Parser;
use palette::LinSrgb;
use rand::Rng;
use std::{
    error::Error,
    io::{self, Write},
    time::Duration,
};
use tokio::{signal, time::sleep};

use hueble_rs::{
    gamut::PHILIPS_GAMUT_C,
    gatt_manipulator::GattManipulator,
    lamp::Lamp,
    wallpaper::{self, ColorPercentPair},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let colors: Vec<ColorPercentPair> = wallpaper::get_dominant_colors(10)?;
    let lin_colors: Vec<LinSrgb> = colors.iter().map(|(srgb, _p)| srgb.into_format()).collect();

    //colors.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));

    for (c, p) in &colors {
        print_color_with_percentage(c, *p);
    }

    let lamp_mac = "D1:DC:EE:F7:7E:8E";
    let conn = GattManipulator::new(lamp_mac).await?;
    let lamp = Lamp::new(&conn, PHILIPS_GAMUT_C);
    lamp.set_power(true).await;

    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    println!("Press Ctrl-C to exit...");

    let mut color_stream = lin_colors.iter().cycle();
    let mut current_color = color_stream.next().unwrap();
    lamp.set_color(current_color).await;

    loop {
        let next_color = color_stream.next().unwrap();

        let max_transition_duration = 10000;
        let max_step_duration = 100;
        let steps = max_transition_duration / max_step_duration;

        for step in 0..steps {
            let t = step as f32 / steps as f32;

            let srgb_color: LinSrgb = interpolate_colors(current_color, next_color, t);
            print_color(&srgb_color);

            tokio::select! {
                _ = &mut ctrl_c => {
                    println!("\nCtrl-C received. Disconnecting...");
                    conn.disconnect().await;
                    return Ok(());
                }
                _ = lamp.set_color(&srgb_color) => {
                    sleep(Duration::from_millis(max_step_duration)).await;
                }
            }
        }

        current_color = next_color;
    }
}

fn print_color_with_percentage(c: &LinSrgb<u8>, percentage: f32) {
    let display_value = format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue);
    println!(
        "\x1B[38;2;{};{};{}m▇ {} {}\x1B[0m",
        c.red, c.green, c.blue, display_value, percentage
    );
}

// Prints a colored box without the newline, for smearing the interpolated color across the
// terminal
fn print_color(c: &LinSrgb) {
    let uc: LinSrgb<u8> = c.into_format();
    print!("\x1B[38;2;{};{};{}m▇\x1B[0m", uc.red, uc.green, uc.blue);
    io::stdout().flush().unwrap();
}

fn interpolate_colors(c1: &LinSrgb, c2: &LinSrgb, factor: f32) -> LinSrgb {
    let t = factor.clamp(0.0, 1.0);

    LinSrgb::new(
        c1.red + (c2.red - c1.red) * t,
        c1.green + (c2.green - c1.green) * t,
        c1.blue + (c2.blue - c1.blue) * t,
    )
}
