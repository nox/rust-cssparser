use ast::*;
use super::to_ascii_lower;
use self::color_data::{COLOR_KEYWORDS, COLOR_VALUES};

mod color_data;

pub type ColorFloat = f64;


pub enum Color {
    CurrentColor,
    RGBA(ColorFloat, ColorFloat, ColorFloat, ColorFloat),  // 0..1
}


// Return None on invalid/unsupported value (not a color)
pub fn parse_color(component_value: &ComponentValue) -> Option<Color> {
    match *component_value {
        Hash(ref value) | IDHash(ref value) => parse_color_hash(*value),
        Ident(ref value) => parse_color_keyword(*value),
        Function(ref name, ref arguments) => parse_color_function(*name, *arguments),
        _ => None
    }
}


#[inline]
fn parse_color_keyword(value: &str) -> Option<Color> {
    let lower_value = to_ascii_lower(value);
    match COLOR_KEYWORDS.bsearch_elem(&lower_value.as_slice()) {
        Some(index) => Some(COLOR_VALUES[index]),
        None => if "currentcolor" == lower_value { Some(CurrentColor) }
                else { None }
    }
}


#[inline]
fn parse_color_hash(value: &str) -> Option<Color> {
    macro_rules! from_hex(
        ($c: expr) => {{
            let c = $c;
            match c as char {
                '0' .. '9' => c - ('0' as u8),
                'a' .. 'f' => c - ('a' as u8) + 10,
                'A' .. 'F' => c - ('A' as u8) + 10,
                _ => return None  // Not a valid color
            }
        }};
    )
    macro_rules! to_rgba(
        ($r: expr, $g: expr, $b: expr,) => {
            Some(RGBA($r as ColorFloat / 255., $g as ColorFloat / 255.,
                      $b as ColorFloat / 255., 1.))
        };
    )

    match value.len() {
        6 => to_rgba!(
            from_hex!(value[0]) * 16 + from_hex!(value[1]),
            from_hex!(value[2]) * 16 + from_hex!(value[3]),
            from_hex!(value[4]) * 16 + from_hex!(value[5]),
        ),
        3 => to_rgba!(
            from_hex!(value[0]) * 17,
            from_hex!(value[1]) * 17,
            from_hex!(value[2]) * 17,
        ),
        _ => None
    }
}


#[inline]
fn parse_color_function(name: &str, arguments: &[(ComponentValue, SourceLocation)])
                        -> Option<Color> {
    let lower_name = to_ascii_lower(name);

    let (is_rgb, has_alpha) =
        if "rgba" == lower_name { (true, true) }
        else if "rgb" == lower_name { (true, false) }
        else if "hsl" == lower_name { (false, false) }
        else if "hsla" == lower_name { (false, true) }
        else { return None };

    let mut iter = do arguments.iter().filter_map |&(ref c, _)| {
        if c != &WhiteSpace { Some(c) } else { None }
    };
    macro_rules! expect_comma(
        () => ( if iter.next() != Some(&Comma) { return None } );
    )
    macro_rules! expect_percentage(
        () => ( match iter.next() {
            Some(&Percentage(ref v)) => v.value,
            _ => return None,
        });
    )
    macro_rules! expect_integer(
        () => ( match iter.next() {
            Some(&Number(ref v)) if v.int_value.is_some() => v.value,
            _ => return None,
        });
    )
    macro_rules! expect_number(
        () => ( match iter.next() {
            Some(&Number(ref v)) => v.value,
            _ => return None,
        });
    )

    let red: ColorFloat;
    let green: ColorFloat;
    let blue: ColorFloat;
    if is_rgb {
        // Either integers or percentages, but all the same type.
        match iter.next() {
            Some(&Number(ref v)) if v.int_value.is_some() => {
                red = v.value / 255.;
                expect_comma!();
                green = expect_integer!() / 255.;
                expect_comma!();
                blue = expect_integer!() / 255.;
            }
            Some(&Percentage(ref v)) => {
                red = v.value / 100.;
                expect_comma!();
                green = expect_percentage!() / 100.;
                expect_comma!();
                blue = expect_percentage!() / 100.;
            }
            _ => return None
        };
    } else {
        let hue: ColorFloat = expect_number!() / 360.;
        let hue = hue - hue.floor();
        expect_comma!();
        let saturation: ColorFloat = (expect_percentage!() / 100.).max(&0.).min(&1.);
        expect_comma!();
        let lightness: ColorFloat = (expect_percentage!() / 100.).max(&0.).min(&1.);

        // http://www.w3.org/TR/css3-color/#hsl-color
        fn hue_to_rgb(m1: ColorFloat, m2: ColorFloat, mut h: ColorFloat) -> ColorFloat {
            if h < 0. { h += 1. }
            if h > 1. { h -= 1. }

            if h * 6. < 1. { m1 + (m2 - m1) * h * 6. }
            else if h * 2. < 1. { m2 }
            else if h * 3. < 2. { m1 + (m2 - m1) * (2. / 3. - h) * 6. }
            else { m1 }
        }
        let m2 = if lightness <= 0.5 { lightness * (saturation + 1.) }
                 else { lightness + saturation - lightness * saturation };
        let m1 = lightness * 2. - m2;
        red = hue_to_rgb(m1, m2, hue + 1. / 3.);
        green = hue_to_rgb(m1, m2, hue);
        blue = hue_to_rgb(m1, m2, hue - 1. / 3.);
    }

    let alpha = if has_alpha {
        expect_comma!();
        match iter.next() {
            Some(&Number(ref a)) => a.value.max(&0.).min(&1.),
            _ => return None
        }
    } else {
        1.
    };
    if iter.next().is_none() { Some(RGBA(red, green, blue, alpha)) } else { None }
}
