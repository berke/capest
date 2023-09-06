use serde::{Serialize,Deserialize};
use std::{
    fs::File,
    path::Path
};

use crate::{
    common::*,
    math::*
};

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct Point {
    pub x:Real,
    pub y:Real
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct Rectangle {
    pub p0:Point,
    pub p1:Point
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct Layer {
    pub name:String,
    pub bitmap:String,
    pub gerber:String,
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct Config {
    pub input:String,
    pub layers:Vec<Layer>,
    pub roi:Option<Rectangle>,
    pub mark:Option<Point>,
    pub output:String,
    pub origin:Point,
    pub dpi:Real,
    pub eps_rel:Real,
    pub thickness:Real,
    pub cap_min:Real
}

pub trait Loadable {
    fn load<P:AsRef<Path>>(path:P)->Res<Self>
    where Self:Sized,for<'a> Self:Deserialize<'a> {
	let fd = File::open(path)?;
	let this : Self = ron::de::from_reader(fd)?;
	Ok(this)
    }
}

impl Loadable for Config { }
