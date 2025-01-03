use palette::{convert::FromColorUnclamped, stimulus::FromStimulus, FromColor, LinSrgb, Yxy};
use uuid::Uuid;

use crate::{
    gamut::{ClampToGamut, PhilipsGamut},
    gatt_manipulator::GattManipulator,
};

// GATT characteristic UUIDs for the hue
const CHAR_MODEL: Uuid = Uuid::from_u128(0x00002a24_0000_1000_8000_00805f9b34fb);
const CHAR_POWER: Uuid = Uuid::from_u128(0x932c32bd_0002_47a2_835a_a8d455b859dd);
const CHAR_BRIGHTNESS: Uuid = Uuid::from_u128(0x932c32bd_0003_47a2_835a_a8d455b859dd);
const CHAR_COLOR: Uuid = Uuid::from_u128(0x932c32bd_0005_47a2_835a_a8d455b859dd);

pub struct Lamp<'a> {
    gattm: &'a GattManipulator,
    gamut: PhilipsGamut,
}

impl<'a> Lamp<'a> {
    pub fn new(conn: &'a GattManipulator, gamut: PhilipsGamut) -> Self {
        Self { gattm: conn, gamut }
    }

    pub async fn set_power(&self, on: bool) {
        let data = [if on { 1u8 } else { 0u8 }];
        self.gattm.write_characteristic(&CHAR_POWER, &data).await;
    }

    /// Set the lamp color and brightness from an sRGB without gamma correction, Hue corrects gamma
    pub async fn set_color<C>(&self, rgb_color: &LinSrgb<C>)
    where
        f32: FromStimulus<C>,
        C: Copy,
    {
        // Convert to linear to undo gamma correction, the Hue corrects gamma
        let xy_luma = Yxy::from_color(rgb_color.into_format::<f32>()).clamp_to(&self.gamut);
        self.set_xy(xy_luma.x, xy_luma.y).await;
        self.set_brightness(xy_luma.luma / max_luminance(&xy_luma))
            .await;
    }

    /// Set raw x,y chrominance
    pub async fn set_xy(&self, x: f32, y: f32) {
        let x_val = (x.clamp(0.0, 1.0) * 65535.0).round() as u16;
        let y_val = (y.clamp(0.0, 1.0) * 65535.0).round() as u16;

        let mut buf = [0u8; 4];
        buf[..2].copy_from_slice(&x_val.to_le_bytes());
        buf[2..4].copy_from_slice(&y_val.to_le_bytes());

        self.gattm.write_characteristic(&CHAR_COLOR, &buf).await;
    }

    /// Set raw lamp brightness
    pub async fn set_brightness(&self, brightness: f32) {
        let raw_val = (brightness.clamp(0.0, 1.0) * 255.0).round() as u8;
        let val_clamped = raw_val.clamp(1, 245);
        self.gattm
            .write_characteristic(&CHAR_BRIGHTNESS, &[val_clamped])
            .await;
    }

    pub async fn get_model(&self) -> String {
        match self.gattm.read_characteristic(&CHAR_MODEL).await {
            Some(model_bytes) => String::from_utf8_lossy(&model_bytes).to_string(),
            None => "".to_string(),
        }
    }
}

/// Find the maximum luminance value that fits into sRGB for a given xy chrominance
fn max_luminance(xyy: &Yxy) -> f32 {
    // Replace Y for our given xyY color with maximum luminance. Then convert to a linear RGB, and
    // then scale all channels by the overshoot. Then convert back to xyY and return Y as the
    // maximum displayable luminiance for the chrominance input.
    let rgb: LinSrgb<f32> = LinSrgb::from_color_unclamped(Yxy::new(xyy.x, xyy.y, 1.0));
    let s = rgb.red.max(rgb.green).max(rgb.blue);
    Yxy::from_color(LinSrgb::new(rgb.red / s, rgb.green / s, rgb.blue / s)).luma
}
