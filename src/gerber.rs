use std::fs::File;
use std::path::Path;
use std::io::{Read,Write,BufWriter,BufReader};
use std::collections::BTreeMap;
use regex::Regex;

use crate::common::*;

pub struct Image {
    pub commands:Vec<Command>
}

#[derive(Debug,Clone)]
pub enum Command {
    DefineAttribute {
	target:AttributeTarget,
	name:String,
	values:Vec<String>
    },
    DeleteAttribute {
	name:Option<String>
    },
    Operation {
	op:Operation,
	x:i32,
	y:i32
    },
    SetCoordinateFormat{ x:CoordinateFormat,
                         y:CoordinateFormat },
    SetAperture(u32),
    DefineAperture {
	code:u32,
	template:String,
	params:Vec<f64>
    },
    ApertureMacro {
	name:String,
	contents:Vec<ApertureMacroContent>
    },
    LoadPolarity(Polarity),
    SetMode(Mode),
    Interpolation(InterpolationMode),
    BeginRegion,
    EndRegion,
    Comment(String),
    EOF,
    Unknown
}

#[derive(Debug,Clone)]
pub struct Point {
    pub x:f64,
    pub y:f64
}

#[derive(Debug,Clone)]
pub enum Binop {
    Add,
    Sub,
    Mul,
    Div
}

#[derive(Debug,Clone)]
pub enum ArithmeticExpr {
    Const(f64),
    Var(u32),
    Binop(Binop,Box<Self>,Box<Self>)
}

impl TryFrom<&str> for ArithmeticExpr {
    type Error = Box<dyn Error>;
    fn try_from(u:&str)->Res<ArithmeticExpr> {
	Ok(Self::Const(0.0))
    }
}

#[derive(Debug,Clone)]
pub enum ApertureMacroContent {
    DefineVar {
	name:u32,
	value:ArithmeticExpr
    },
    Primitive {
	code:u32,
	modifiers:Vec<ArithmeticExpr>
    },
    // Circle {
    // 	exposure:bool,
    // 	diameter:f64,
    // 	center:Point,
    // 	rotation:Option<f64>
    // },
    // VectorLine {
    // 	exposure:bool,
    // 	line_width:f64,
    // 	start:Point,
    // 	end:Point,
    // 	rotation:f64
    // },
    // CenterLine {
    // 	exposure:bool,
    // 	width:f64,
    // 	height:f64,
    // 	center:Point,
    // 	rotation:f64
    // },
    // Outline {
    // 	exposure:bool,
    // 	vertices:Vec<Point>,
    // 	rotation:f64
    // },
    // Polygon {
    // 	exposure:bool,
    // 	num_vertices:u32,
    // 	center:Point,
    // 	diameter:f64,
    // 	rotation:f64
    // },
    // Moire {
    // 	center:Point,
    // 	outer_diameter:f64,
    // 	ring_thickness:f64,
    // 	ring_gap:f64,
    // 	max_num_rings:u32,
    // 	crosshair_thickness:f64,
    // 	crosshair_length:f64,
    // 	rotation:f64
    // },
    // Thermal {
    // 	center:Point,
    // 	outer_diameter:f64,
    // 	inner_diameter:f64,
    // 	gap_thickness:f64,
    // 	rotation:f64
    // },
    Comment(String),
}

#[derive(Debug,Clone)]
pub enum ApertureTemplate {
    Circle { diameter:f64,hole_diameter:Option<f64> },
    Rectangle { x_size:f64,y_size:f64,hole_diameter:Option<f64> },
    Obround { x_size:f64,y_size:f64,hole_diameter:Option<f64> },
    Polygon { outer_diameter:f64,num_vertices:u32,rotation:Option<f64>,
	      hole_diameter:Option<f64> }
}

#[derive(Debug,Clone)]
pub enum Operation {
    Move,
    Interpolate,
    Flash
}

impl From<&str> for Operation {
    fn from(x:&str)->Self {
	match x {
	    "01" => Self::Interpolate,
	    "02" => Self::Move,
	    "03" => Self::Flash,
	    _ => panic!("Invalid operation")
	}
    }
}

#[derive(Debug,Clone,Copy)]
pub struct CoordinateFormat {
    integer:u8,
    decimal:u8
}

impl Default for CoordinateFormat {
    fn default()->Self {
	Self {
	    integer:6,
	    decimal:6
	}
    }
}

impl CoordinateFormat {
    pub fn convert(&self,d:i32)->f64 {
	d as f64 / 10.0_f64.powi(self.decimal as i32)
    }
}

impl From<u8> for CoordinateFormat {
    fn from(f:u8)->Self {
	Self {
	    integer:f / 10,
	    decimal:f % 10
	}
    }
}

#[derive(Debug,Clone)]
pub enum AttributeTarget {
    File,
    Aperture,
    Object
}

impl From<&str> for AttributeTarget {
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
pub enum Polarity {
    Dark,
    Clear,
}

impl From<&str> for Polarity {
    fn from(x:&str)->Self {
	match x {
	    "D" => Self::Dark,
	    "C" => Self::Clear,
	    _ => panic!("Invalid polarity")
	}
    }
}

#[derive(Debug,Clone)]
pub enum Mode {
    Inches,
    Millimeters
}

impl From<&str> for Mode {
    fn from(x:&str)->Self {
	match x {
	    "MM" => Self::Millimeters,
	    "IN" => Self::Inches,
	    _ => panic!("Invalid mode")
	}
    }
}

#[derive(Debug,Clone)]
pub enum InterpolationMode {
    Linear,
    CircularClockwise,
    CircularCounterClockwise,
    CircularSingleQuadrant,
    CircularMultiQuadrant
}

impl From<&str> for InterpolationMode {
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

pub struct NetInfos {
    pub index:BTreeMap<String,Vec<Point>>
}

impl From<&Image> for NetInfos {
    fn from(img:&Image)->Self {
	let mut index : BTreeMap<String,Vec<Point>> = BTreeMap::new();
	let mut net : Option<&str> = None;
	let mut scale_x = 1.0;
	let mut scale_y = 1.0;
	let mut x_cf = CoordinateFormat::default();
	let mut y_cf = CoordinateFormat::default();
	for cmd in &img.commands {
	    match cmd {
		Command::SetMode(Mode::Inches) => {
		    scale_x = 25.4;
		    scale_y = scale_x;
		},
		Command::SetMode(Mode::Millimeters) => {
		    scale_x = 1.0;
		    scale_y = scale_x;
		},
		&Command::SetCoordinateFormat { x, y } => {
		    x_cf = x;
		    y_cf = y;
		},
		Command::DefineAttribute {
		    target:AttributeTarget::Object,
		    name,
		    values
		} if name == ".N" && values.len() == 1 => {
		    net = Some(&values[0]);
		},
		Command::DeleteAttribute {
		    name
		} => {
		    if match name {
			None => true,
			Some(u) if u == ".N" => true,
			_ => false
		    } {
			net = None;
		    }
		},
		&Command::Operation {
		    op:Operation::Flash,
		    x,
		    y
		} => {
		    if let Some(name) = net {
			let mut v = index
			    .entry(name.to_string())
			    .or_insert_with(|| Vec::new());
			let x = x_cf.convert(x);
			let y = y_cf.convert(y);
			v.push(Point{ x, y });
		    }
		}
		_ => ()
	    }
	}
	Self { index }
    }
}


impl Image {
    fn remove_crlf(u:&str)->String {
	u.chars()
	    .filter(|&c| c != '\r' && c != '\n')
	    .collect()
    }

    fn aperture_macro_contents_from_str(u:&str)->
	Res<Vec<ApertureMacroContent>> {
	let mut contents : Vec<ApertureMacroContent> = Vec::new();
	let comment_rex = Regex::new(r"^0 (.*)$")?;
	let var_def_rex = Regex::new(r"^\$([0-9]+)=(.*)$")?;
	let prim_rex = Regex::new(r"^([0-9]+),(.*)$")?;

	for v in u.split('*') {
	    let co =
		if let Some(caps) = comment_rex.captures(&v) {
		    Some(ApertureMacroContent::Comment(caps[1].into()))
		} else if let Some(caps) = var_def_rex.captures(&v) {
		    let name : u32 = caps[1].parse()?;
		    let value : ArithmeticExpr = caps[2].try_into()?;
		    Some(
			ApertureMacroContent::DefineVar {
			    name,
			    value
			})
		} else if let Some(caps) = prim_rex.captures(&v) {
		    let code : u32 = caps[1].parse()?;
		    let mut modifiers = Vec::new();
		    for v in caps[2].split(',') {
			let x : ArithmeticExpr = v.try_into()?;
			modifiers.push(x);
		    }
		    Some(
			ApertureMacroContent::Primitive {
			    code,
			    modifiers
			})
		} else {
		    println!("? AMC {}",v);
		    None
		};
	    if let Some(c) = co {
		contents.push(c);
	    }
	}
	Ok(contents)
    }

    pub fn parse(u:&str)->Res<Self> {
	let mut commands : Vec<Command> = Vec::new();
	
	let block_rex = Regex::new(r"([^%*]+)\*|%([^%]+)\*%")?;
	let op_rex = Regex::new(r"^X([+-]?[0-9]+)Y([+-]?[0-9]+)D([0-9]{2})$")?; // XXX: X,Y or XY
	let del_attr_rex = Regex::new(r"^TD(.+)?$")?;
	let attr_rex = Regex::new(r"^T([FAO])([^,]+)((,[^,]+)*)$")?;
	let comment_rex = Regex::new(r"^G04 (.*)$")?;
	let mode_rex = Regex::new(r"^MO(MM|IN)$")?;
	let aperture_rex = Regex::new(r"^D([1-9][0-9]+)$")?;
	let decimal = r"[+-]?(:?[0-9]+(:?\.[0-9]*)?|\.[0-9]+)";
	let def_aperture_rex = Regex::new(
	    &format!(r"^ADD([1-9][0-9]+)([A-Za-z0-9]+),({decimal}(:?X{decimal})*)$"))?;
	let circ_rex = Regex::new(&format!("^{decimal}(:?X({decimal}))?$"))?;
	let rect_rex =
	    Regex::new(&format!("^{decimal}X({decimal})(:?X({decimal}))?$"))?;
	let obr_rex =
	    Regex::new(&format!("^{decimal}X({decimal})(:?X({decimal}))?$"))?;
	let fs_rex = Regex::new(r"^FSLAX([0-9]{2})Y([0-9]{2})$")?;
	let lp_rex = Regex::new(r"^LP([DC])$")?;
	let am_rex = Regex::new(r"^AM([A-Za-z0-9]+)\*(.*)$")?;

	for caps in block_rex.captures_iter(u) {
	    let cmd =
		if let Some(cmd) = caps.get(1) {
		    let cmd = Self::remove_crlf(cmd.as_str());
		    if let Some(caps) = op_rex.captures(&cmd) {
			let x : i32 = caps[1].parse()?;
			let y : i32 = caps[2].parse()?;
			let op : Operation = caps[3].into();
			Some(Command::Operation {
			    x,
			    y,
			    op
			})
		    } else if let Some(caps) = comment_rex.captures(&cmd) {
			Some(Command::Comment(caps[1].into()))
		    } else if let Some(caps) = aperture_rex.captures(&cmd) {
			let d : u32 = caps[1].parse()?;
			Some(Command::SetAperture(d))
		    } else {
			match cmd.as_str() {
			    "M02" => Some(Command::EOF),
			    "G36" => Some(Command::BeginRegion),
			    "G37" => Some(Command::EndRegion),
			    u @ ("G01"|"G02"|"G03"|"G74"|"G75") =>
				Some(Command::Interpolation(
				    u.trim_start_matches('G').into())),
			    _ => None
			}
		    }
		} else if let Some(cmd) = caps.get(2) {
		    // Extended commands
		    
		    let cmd = Self::remove_crlf(cmd.as_str());

		    if let Some(caps) = attr_rex.captures(&cmd) {
			// println!("  T1 {}",&caps[1]);
			let target : AttributeTarget = caps[1].into();
			let name : String = caps[2].into();
			let values : Vec<String> = caps[3]
			    .trim_start_matches(',')
			    .split(',')
			    .map(|x| x.to_string())
			    .collect();
			Some(Command::DefineAttribute {
			    target,
			    name,
			    values
			})
		    } else if let Some(caps) = fs_rex.captures(&cmd) {
			let x : u8 = caps[1].parse()?;
			let y : u8 = caps[1].parse()?;
			Some(Command::SetCoordinateFormat {
			    x:x.into(),
			    y:y.into()
			})
		    } else if let Some(caps) = mode_rex.captures(&cmd) {
			Some(Command::SetMode(caps[1].into()))
		    } else if let Some(caps) = del_attr_rex.captures(&cmd) {
			let name : Option<String> = caps.get(1).map(|x| x.as_str().into());
			Some(Command::DeleteAttribute { name })
		    } else if let Some(caps) = lp_rex.captures(&cmd) {
			Some(Command::LoadPolarity(caps[1].into()))
		    } else if let Some(caps) = am_rex.captures(&cmd) {
			let name : String = caps[1].into();
			let macro_def : &str = &caps[2];
			let contents =
			    Self::aperture_macro_contents_from_str(macro_def)?;
			Some(Command::ApertureMacro {
			    name,
			    contents
			})
		    } else if let Some(caps) = def_aperture_rex.captures(&cmd) {
			let code : u32 = caps[1].parse()?;
			let template = caps[2].to_string();
			let params : Vec<f64> =
			    caps[3]
			    .split('X')
			    .map(|x| x.parse().unwrap())
			    .collect();
			Some(Command::DefineAperture {
			    code,
			    template,
			    params
			})
		    } else {
			None
		    }
		} else {
		    None
		};
	    commands.push(cmd.unwrap_or(Command::Unknown));
	}
	Ok(Self { commands })
    }

    pub fn from_file<P:AsRef<Path>>(path:P)->Res<Self> {
	let mut fd = File::open(path)?;
	let mut u = String::new();
	let m = fd.read_to_string(&mut u)?;
	Ok(Self::parse(&u)?)
    }
}
