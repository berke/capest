mod math;
mod math_random;
mod common;
mod gerber;

use std::{
    fs::File,
    io::Read
};

use pico_args::Arguments;

use common::*;
use gerber::Image;

fn main()->Res<()> {
    let mut args = Arguments::from_env();
    let fn_in : String = args.value_from_str("--input")?;
    let mut fd = File::open(&fn_in)?;
    let mut u = String::new();
    let m = fd.read_to_string(&mut u)?;
    println!("Read {} bytes",m);

    let gbr = Image::parse(&u)?;
    println!("{:#?}",gbr.commands);
    Ok(())
}
