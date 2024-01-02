use std::cell::RefCell;
use std::{thread, time};

use rand::prelude::*;
use ws2818_rgb_led_spi_driver::adapter_gen::WS28xxAdapter;
use ws2818_rgb_led_spi_driver::adapter_spi::WS28xxSpiAdapter;
use ws2818_rgb_led_spi_driver::encoding::encode_rgb;

/*

Digit layout:

 1111
6    2
6    2
 7777
5    3
5    3
 4444   8

*/
const ZERO: u8 = 0b00111111;
const ONE: u8 = 0b00000110;
const TWO: u8 = 0b01011011;
const THREE: u8 = 0b01001111;
const FOUR: u8 = 0b01100110;
const FIVE: u8 = 0b01101101;
const SIX: u8 = 0b01111101;
const SEVEN: u8 = 0b00000111;
const EIGHT: u8 = 0b01111111;
const NINE: u8 = 0b01101111;

const MINUS: u8 = 0b01000000;

trait WriteRgbDigit {
    fn write_spi_encoded(&mut self, encoded: &Vec<u8>) -> Result<(), String>;
}

impl WriteRgbDigit for WS28xxSpiAdapter {
    fn write_spi_encoded(&mut self, encoded: &Vec<u8>) -> Result<(), String> {
        let encoded: Vec<u8> = encoded
            .chunks(3)
            .flat_map(|chunk| {
                let [r, g, b]: [_; 3] = chunk
                    .try_into()
                    .expect("should be chunks of three for each r/g/b channel!");
                encode_rgb(r, g, b)
            })
            .collect();

        self.write_encoded_rgb(&encoded)
    }
}

#[derive(Clone)]
struct SevenSegmentDisplay {
    state_rgb: [u8; 24],
}

pub(crate) struct SevenSegmentDisplayString {
    digits: Vec<RefCell<SevenSegmentDisplay>>,
    adapter: RefCell<Box<dyn WriteRgbDigit>>,
}

impl SevenSegmentDisplayString {
    pub(crate) fn new(
        adapter: impl WriteRgbDigit + 'static,
        display_count: usize,
    ) -> SevenSegmentDisplayString {
        // let display_init: [u8; 24] = random();
        let display_init: [u8; 24] = [0; 24];
        let new_display_state = SevenSegmentDisplay {
            state_rgb: display_init,
        };

        let digits = vec![new_display_state; display_count]
            .into_iter()
            .map(RefCell::new)
            .collect();

        return SevenSegmentDisplayString {
            digits,
            adapter: RefCell::new(Box::new(adapter)),
        };
    }

    pub fn flush(&self) {
        let encoded: Vec<u8> = self
            .digits
            .iter()
            .flat_map(|it| it.borrow().state_rgb)
            .collect();

        self.adapter
            .borrow_mut()
            .write_spi_encoded(&encoded)
            .expect("should work");
    }

    pub fn derive_numeric_display(&self, display_indices: &[usize]) -> NumericDisplay {
        let digits = display_indices
            .into_iter()
            .map(|i| &self.digits[*i])
            .collect();

        return NumericDisplay {
            digits,
            value: None,
            color_rgb: (0, 0, 0),
        };
    }
}

trait NumericSevenSegmentDisplay {
    fn set_digit(&mut self, value: SevenSegmentChar, color: (u8, u8, u8), decimal: bool);
}

#[derive(Clone, Debug)]
enum SevenSegmentChar {
    Number(u8),
    Minus,
    BLANK,
}

impl NumericSevenSegmentDisplay for SevenSegmentDisplay {
    fn set_digit(&mut self, char: SevenSegmentChar, color: (u8, u8, u8), decimal: bool) {
        let mut encoded = match char {
            SevenSegmentChar::Number(value) => match value {
                0 => ZERO,
                1 => ONE,
                2 => TWO,
                3 => THREE,
                4 => FOUR,
                5 => FIVE,
                6 => SIX,
                7 => SEVEN,
                8 => EIGHT,
                9 => NINE,
                _ => panic!("Single digits only allowed! [{} sent]", value),
            },

            SevenSegmentChar::Minus => MINUS,
            SevenSegmentChar::BLANK => 0,
        };

        if decimal {
            encoded = encoded | 0b10000000;
        }

        let mut led_colors: [u8; 24] = [0; 24];
        let (r, g, b) = color;
        let color_slice = [r, g, b];

        for i in 0..8 {
            if encoded >> i & 1 == 1 {
                let offset = i * 3;

                led_colors[offset..offset + 3].copy_from_slice(&color_slice);
            }
        }

        self.state_rgb = led_colors
    }
}

pub(crate) struct NumericDisplay<'a> {
    digits: Vec<&'a RefCell<SevenSegmentDisplay>>,
    value: Option<String>,
    color_rgb: (u8, u8, u8),
}

impl NumericDisplay<'_> {
    pub fn clear(&mut self) {
        self.value = None;
    }

    pub fn set_color(&mut self, color_rgb: (u8, u8, u8)) {
        self.color_rgb = color_rgb;
    }

    pub fn set_value(&mut self, value: String) {
        self.value = Some(value);
    }

    pub fn write(&mut self) -> Result<(), String> {
        let chars: Vec<(SevenSegmentChar, bool)> = match &self.value {
            None => {
                vec![(SevenSegmentChar::BLANK, false); self.digits.len()]
            }
            Some(value) => {
                let mut chars_iter = value.chars().peekable();

                let mut chars = vec![];

                while let Some(c) = chars_iter.next() {
                    let decimal = chars_iter.peek() == Some(&'.');
                    if decimal {
                        chars_iter.next(); // consume the decimal
                    }

                    let char = match c {
                        '0'..='9' => SevenSegmentChar::Number(
                            c.to_digit(10)
                                .expect("Char should map to digit")
                                .try_into()
                                .expect("Char should map to u8"),
                        ),
                        '-' => SevenSegmentChar::Minus,
                        ' ' => SevenSegmentChar::BLANK,
                        _ => panic!("Unsupported char {c}"), // @todo make the type a Result
                    };

                    chars.push((char, decimal))
                }

                chars
            }
        };

        if &chars.len() > &self.digits.len() {
            return Err(format!(
                "Insufficient digits to display value [{:?}]",
                &self.value
            ));
        }

        for (idx, (char, decimal)) in chars.into_iter().enumerate() {
            self.digits[idx]
                .borrow_mut()
                .set_digit(char, self.color_rgb, decimal)
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::rgbdigit::SevenSegmentDisplayString;
    use std::{thread, time};
    use ws2818_rgb_led_spi_driver::adapter_spi::WS28xxSpiAdapter;

    #[test]
    fn it_works() {
        let adapter = WS28xxSpiAdapter::new("/dev/spidev0.0").unwrap();

        let display_string = SevenSegmentDisplayString::new(adapter, 4);

        let mut first_pair = display_string.derive_numeric_display(&[0, 1]);
        let mut second_pair = display_string.derive_numeric_display(&[2, 3]);

        let first_grad = colorgrad::rainbow();
        let second_grad = colorgrad::viridis();

        first_pair.set_color((105, 68, 5));
        second_pair.set_color((40, 5, 45));

        loop {
            for i in 1..=99 {
                let (r, g, b, _) = first_grad.at(i as f64 / 100.0).to_linear_rgba_u8();
                first_pair.set_color((r, g, b));
                first_pair.set_value(format!("{i:>2}"));
                first_pair.write()?;

                let decimal = i as f32 / 10.0;
                let (r, g, b, _) = second_grad.at(i as f64 / 100.0).to_linear_rgba_u8();
                second_pair.set_color((r, g, b));
                second_pair.set_value(format!("{decimal:.1}"));
                second_pair.write()?;

                display_string.flush();

                thread::sleep(time::Duration::from_millis(100));
            }
        }
    }
}
