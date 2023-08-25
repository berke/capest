mod math;
mod math_random;

use std::collections::BTreeSet;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::io::{Read,Write,BufWriter,BufReader};
use regex::Regex;
use math::*;

use pico_args::Arguments;

type Res<T> = Result<T,Box<dyn Error>>;

fn error(msg:&str)->Box<dyn Error> {
    Box::new(std::io::Error::new(std::io::ErrorKind::Other,msg))
}

struct Gerber {
    commands:Vec<GerberCommand>
}

#[derive(Debug,Clone)]
enum GerberCommand {
    DefineAttribute {
	target:GerberAttributeTarget,
	name:String,
	values:Vec<String>
    },
    DeleteAttribute {
	name:Option<String>
    },
    Operation {
	op:GerberOperation,
	x:i32,
	y:i32
    },
    SetCoordinateFormat{ x:GerberCoordinateFormat,
                         y:GerberCoordinateFormat },
    SetAperture(u32),
    SetMode(GerberMode),
    Interpolation(GerberInterpolationMode),
    Comment(String)
}

#[derive(Debug,Clone)]
enum GerberOperation {
    Move,
    Interpolate,
    Flash
}

impl From<&str> for GerberOperation {
    fn from(x:&str)->Self {
	match x {
	    "01" => Self::Interpolate,
	    "02" => Self::Move,
	    "03" => Self::Flash,
	    _ => panic!("Invalid operation")
	}
    }
}

#[derive(Debug,Clone)]
struct GerberCoordinateFormat {
    integer:u8,
    decimal:u8
}

impl From<u8> for GerberCoordinateFormat {
    fn from(f:u8)->Self {
	Self {
	    integer:f / 10,
	    decimal:f % 10
	}
    }
}

#[derive(Debug,Clone)]
enum GerberAttributeTarget {
    File,
    Aperture,
    Object
}

impl From<&str> for GerberAttributeTarget {
    fn from(x:&str)->Self {
	match x {
	    "F" => Self::File,
	    "A" => Self::Aperture,
	    "O" => Self::Object,
	    _ => panic!("Invalid attribute target")
	}
    }
}

#[derive(Debug,Clone)]
enum GerberMode {
    Inches,
    Millimeters
}

impl From<&str> for GerberMode {
    fn from(x:&str)->Self {
	match x {
	    "MM" => Self::Millimeters,
	    "IN" => Self::Inches,
	    _ => panic!("Invalid mode")
	}
    }
}

#[derive(Debug,Clone)]
enum GerberInterpolationMode {
    Linear,
    CircularClockwise,
    CircularCounterClockwise,
    CircularSingleQuadrant,
    CircularMultiQuadrant
}

impl From<&str> for GerberInterpolationMode {
    fn from(x:&str)->Self {
	match x {
	    "01" => Self::Linear,
	    "02" => Self::CircularClockwise,
	    "03" => Self::CircularCounterClockwise,
	    "74" => Self::CircularSingleQuadrant,
	    "75" => Self::CircularMultiQuadrant,
	    _ => panic!("Invalid interpolation mode")
	}
    }
}

impl Gerber {

    fn remove_crlf(u:&str)->String {
	u.chars()
	    .filter(|&c| c != '\r' && c != '\n')
	    .collect()
    }

    pub fn parse(u:&str)->Res<Self> {
	let mut commands : Vec<GerberCommand> = Vec::new();
	
	// let d_rex = Regex::new(r"^D([0-9]{2})$")?;
	let op_rex = Regex::new(r"^X([+-]?[0-9]+)Y([+-]?[0-9]+)D([0-9]{2})$")?;
	let del_attr_rex = Regex::new(r"^TD(.+)?$")?;
	let attr_rex = Regex::new(r"^T([FAO])([^,]+)((,[^,]+)*)$")?;
	let block_rex = Regex::new(r"([^%*]+)\*|%([^%*]+)\*%")?;
	let comment_rex = Regex::new(r"^G04 (.*)$")?;
	let mode_rex = Regex::new(r"^MO(MM|IN)$")?;
	let aperture_rex = Regex::new(r"^D([1-9][0-9]+)$")?;
	let fs_rex = Regex::new(r"^FSLAX([0-9]{2})Y([0-9]{2})$")?;
	let m = u.len();
	let mut eof = false;

	for caps in block_rex.captures_iter(u) {
	    if eof {
		return Err(error(&format!("Junk at end of file: {}",&caps[0])));
	    }
	    let cmd =
		if let Some(cmd) = caps.get(1) {
		    let cmd = Self::remove_crlf(cmd.as_str());
		    if let Some(caps) = op_rex.captures(&cmd) {
			let x : i32 = caps[1].parse()?;
			let y : i32 = caps[2].parse()?;
			let op : GerberOperation = caps[3].into();
			Some(GerberCommand::Operation {
			    x,
			    y,
			    op
			})
		    } else if let Some(caps) = comment_rex.captures(&cmd) {
			Some(GerberCommand::Comment(caps[1].into()))
		    } else if let Some(caps) = aperture_rex.captures(&cmd) {
			let d : u32 = caps[1].parse()?;
			Some(GerberCommand::SetAperture(d))
		    } else {
			match cmd.as_str() {
			    "M02" => {
				eof = true;
				None
			    },
			    u @ ("G01"|"G02"|"G03"|"G74"|"G75") =>
				Some(GerberCommand::Interpolation(
				    u.trim_start_matches('G').into())),
			    _ => {
				println!("? {}",cmd);
				None
			    }
			}
		    }
		} else if let Some(cmd) = caps.get(2) {
		    // Extended commands
		    
		    let cmd = Self::remove_crlf(cmd.as_str());

		    if let Some(caps) = attr_rex.captures(&cmd) {
			// println!("  T1 {}",&caps[1]);
			let target : GerberAttributeTarget = caps[1].into();
			let name : String = caps[2].into();
			let values : Vec<String> = caps[3]
			    .trim_start_matches(',')
			    .split(',')
			    .map(|x| x.to_string())
			    .collect();
			Some(GerberCommand::DefineAttribute {
			    target,
			    name,
			    values
			})
		    } else if let Some(caps) = fs_rex.captures(&cmd) {
			let x : u8 = caps[1].parse()?;
			let y : u8 = caps[1].parse()?;
			Some(GerberCommand::SetCoordinateFormat {
			    x:x.into(),
			    y:y.into()
			})
		    } else if let Some(caps) = mode_rex.captures(&cmd) {
			Some(GerberCommand::SetMode(caps[1].into()))
		    } else if let Some(caps) = del_attr_rex.captures(&cmd) {
			let name : Option<String> = caps.get(1).map(|x| x.as_str().into());
			Some(GerberCommand::DeleteAttribute { name })
		    } else {
			println!("?X {}",cmd);
			None
		    }
		} else {
		    None
		};
	    if let Some(cmd) = cmd {
		commands.push(cmd);
	    }
	}
	
	// for cmd in u.split('*') {
	//     let v : String = cmd
	// 	.chars()
	// 	.filter(|&c| c != '\r' && c != '\n')
	// 	.collect();
	//     if v.starts_with('%') {
	// 	println!("EXT {:?}",v);
	//     } else {
	// 	println!("NORM {:?}",v);
	// 	if let Some(caps) = xyd_rex.captures(&v) {
	// 	    println!("  X {}",&caps[1]);
	// 	    println!("  Y {}",&caps[2]);
	// 	    println!("  D {}",&caps[3]);
	// 	} else if let Some(caps) = d_rex.captures(&v) {
	// 	    println!("  D {}",&caps[1]);
	// 	} else {
	// 	    println!("NOCAP");
	// 	}
	//     }
	// }
	Ok(Self { commands })
    }
}

fn main()->Res<()> {
    let mut args = Arguments::from_env();
    let fn_in : String = args.value_from_str("--input")?;
    let mut fd = File::open(&fn_in)?;
    let mut u = String::new();
    let m = fd.read_to_string(&mut u)?;
    println!("Read {} bytes",m);

    let gbr = Gerber::parse(&u)?;
    println!("{:#?}",gbr.commands);
    Ok(())
}