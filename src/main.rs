mod math;
mod math_random;
mod interpol;
mod xorwow;
mod spherical;
mod disk;
mod progress;
mod ndarray_image;
mod gerber;
mod common;

use std::collections::BTreeSet;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::io::{Read,Write,BufWriter,BufReader};
// use ndarray::{Array2,Array3,ArrayViewMut2,ArrayViewMut3};
// use rayon::prelude::*;
use math::*;
// use math_random::Random;
// use disk::DiskIterator;
// use progress::ProgressIndicator;
use pico_args::Arguments;

use xorwow::Xorwow;
use gerber::{Image,NetInfos};

use common::*;

fn convert(x:Real)->u8 {
    let x = (255.0 * x + 0.5).floor() as isize;
    x.max(0).min(255) as u8
}

struct Artwork {
    num_layers:usize,
    layers:Array2<u16>
}

impl Artwork {
    pub fn new(lay_fns:&[&str])->Res<Self> {
	let mut layers_opt = None;
	for (ilay,lay_fn) in lay_fns.iter().enumerate() {
	    println!("Loading layer {} from {}",ilay,lay_fn);
	    let img = ndarray_image::open_gray_image(lay_fn)?;
	    let (ny,nx) = img.dim();
	    let mut layers = layers_opt.take()
		.unwrap_or_else(|| Array2::zeros((ny,nx)));
	    let (nyp,nxp) = layers.dim();
	    if ny != nyp || nx != nxp {
		return Err(error(&format!(
		    "Incoherent dimensions: ({},{}) vs ({},{})",
		    nyp,nxp,ny,nx)));
	    }
	    let mask : u16 = 1 << ilay;
	    for iy in 0..ny {
		for ix in 0..nx {
		    let l = img[[iy,ix]];
		    layers[[iy,ix]] |= if l > 0 { mask } else { 0 };
		}
	    }
	    layers_opt = Some(layers);
	}
	let num_layers = lay_fns.len();
	let layers = layers_opt.ok_or_else(|| error("No layers"))?;
	Ok(Self { num_layers,layers })
    }

    pub fn connected_components(&self)->Vec<ConnectedComponents> {
	let mut components = Vec::new();
	for ilay in 0..self.num_layers {
	    let cc = ConnectedComponents::from_array(&self.layers,1 << ilay);
	    components.push(cc);
	}
	components
    }
}

#[derive(Copy,Clone,Debug,PartialEq,PartialOrd,Ord,Eq)]
struct CellId {
    iy:i16,
    ix:i16
}

impl CellId {
    pub fn neighbours(&self)->[Self;4] {
	let &Self { iy,ix } = self;
	[
	    Self { iy:iy - 1, ix },
	    Self { iy:iy + 1, ix },
	    Self { iy:iy, ix:ix - 1 },
	    Self { iy:iy, ix:ix + 1 }
	]
    }
}

impl From<(usize,usize)> for CellId {
    fn from((iy,ix):(usize,usize))->Self {
	Self { iy:iy as i16,ix:ix as i16 }
    }
}

impl Into<(usize,usize)> for CellId {
    fn into(self)->(usize,usize) {
	(self.iy as usize,
	 self.ix as usize)
    }
}

struct ConnectedComponents {
    components:Vec<BTreeSet<CellId>>
}

impl ConnectedComponents {
    pub fn from_array(a:&Array2<u16>,mask:u16)->Self {
	let (ny,nx) = a.dim();
	let mut components = Vec::new();
	let mut remaining : BTreeSet<CellId> = BTreeSet::new();
	let p = (ny*nx + 63) & !63;
	let mut visited : Array1<u64> = Array1::zeros(p);

	for (idx,x) in a.indexed_iter() {
	    if x & mask != 0 {
		remaining.insert(idx.into());
	    }
	}

	let mut component = Vec::new();
	let mut active = Vec::new();

	loop {
	    if let Some(k) = remaining.pop_first() {
		component.clear();
		active.clear();
		active.push(k);
		loop {
		    if let Some(k) = active.pop() {
			let q = (k.iy as usize * nx) + k.ix as usize;
			visited[q >> 6] |= 1 << (q & 63);
			remaining.remove(&k);
			component.push(k);
			for c in k.neighbours() {
			    if c.iy >= 0 && c.ix >= 0 {
				let iy = c.iy as usize;
				let ix = c.ix as usize;
				let r = iy * nx + ix;
				let m = 1 << (r & 63);
				if visited[r >> 6] & m == 0 &&
				    a[[iy,ix]] & mask != 0 {
				    visited[r >> 6] |= m;
				    active.push(c);
				}
			    }
			}
		    } else {
			break;
		    }
		}
		let c : BTreeSet<CellId> = component.iter().cloned().collect();
		components.push(c);
	    } else {
		break;
	    }
	}
	
	Self {
	    components
	}
    }

    pub fn dump(&self) {
	for (icom,com) in self.components.iter().enumerate() {
	    println!("  {:05} {:10}",icom,com.len());
	}
    }
}

fn main()->Res<()> {
    let mut args = Arguments::from_env();

    println!("Layers");
    let lay_fns : Vec<String> = args.values_from_str("--layer")?;
    let lay_fns_str : Vec<&str> = lay_fns.iter().map(|s| s.as_str()).collect();

    let gbr_fns : Vec<String> = args.values_from_str("--gerber")?;
    let gbr_fns_str : Vec<&str> = gbr_fns.iter().map(|s| s.as_str()).collect();

    let artwork = Artwork::new(&lay_fns_str)?;
    let (ny,nx) = artwork.layers.dim();
    println!("Dimensions: {} x {}",ny,nx);

    let dpi : f64 = args.opt_value_from_str("--dpi")?.unwrap_or(600.0);
    let eps_rel : f64 = args.opt_value_from_str("--eps-rel")?.unwrap_or(4.2);
    let thickness : f64 = args.opt_value_from_str("--thickness")?.
	unwrap_or(1.6);
    let cap_min : f64 = args.opt_value_from_str("--cap-min")?.unwrap_or(1.0e-12);
    let x0 : f64 = args.opt_value_from_str("--origin-x")?.unwrap_or(0.0);
    let y0 : f64 = args.opt_value_from_str("--origin-y")?.unwrap_or(0.0);
    let mark_x : Option<f64> = args.opt_value_from_str("--mark-x")?;
    let mark_y : Option<f64> = args.opt_value_from_str("--mark-y")?;

    let cc = artwork.connected_components();
    let nlay = artwork.num_layers;
    
    let delta = 25.4 / dpi;

    let (ny,nx) = artwork.layers.dim();

    let mut xw = Xorwow::new(1);

    // Origin at bottom-left corner
    // Thus
    //
    // X = delta * (ix + 0.5) + X0
    // Y = (ny - iy - 0.5)*delta + Y0

    // ix = (X - X0)/delta - 0.5
    // iy = ny - (Y - Y0)/delta - 0.5

    let mut net_infos = Vec::new();
    for path in &gbr_fns {
	println!("Loading Gerber file {}",path);
	let img = Image::from_file(path)?;
	let infos : NetInfos = (&img).into();
	net_infos.push(infos);
    }

    let mut component_ids_per_layer = Array3::zeros((nlay,ny,nx));
    let mut component_names_per_layer = Vec::new();

    for ilay in 0..nlay {
	let ccs = &cc[ilay];
	let m = ccs.components.len();
	println!("Layer {} components {}",ilay,m);

	let mut palette = Array2::zeros((m,3));
	for i in 0..m {
	    let x = xw.next();
	    palette[[i,0]] = ((x >> 16) & 255) as u8;
	    palette[[i,1]] = ((x >> 8) & 255) as u8;
	    palette[[i,2]] = (x & 255) as u8;
	}

	let mut img : Array3<u8> = Array3::zeros((ny,nx,3));

	for (icom,com) in ccs.components.iter().enumerate() {
	    let r = palette[[icom,0]];
	    let g = palette[[icom,1]];
	    let b = palette[[icom,2]];
	    for &cid in com.iter() {
		let (iy,ix) : (usize,usize) = cid.into();
		img[[iy,ix,0]] = r;
		img[[iy,ix,1]] = g;
		img[[iy,ix,2]] = b;
		component_ids_per_layer[[ilay,iy,ix]] = icom + 1;
	    }
	}

	let mut component_names = vec![None;m];

	// Try to match components
	for (name,points) in net_infos[ilay].index.iter() {
	    // print!("{} -> ",name);
	    for &gerber::Point { x, y } in points.iter() {

		let ixf = ((x - x0)/delta - 0.5).floor();
		let iyf = (ny as f64 - (y - y0)/delta - 0.5).floor();
		// print!("  {},{} ({},{})",x,y,ixf,iyf);
		if 0.0 <= ixf && 0.0 <= iyf {
		    let ix = ixf as usize;
		    let iy = iyf as usize;
		    if ix < nx && iy < ny {
			let icom = component_ids_per_layer[[ilay,iy,ix]];
			if icom > 0 {
			    component_names[icom - 1] = Some(name);
			}
			// print!(":{}",icom);
		    } else {
			// print!("?");
		    }
		} else {
		    // print!("?");
		}
	    }
	    // println!();
	}

	component_names_per_layer.push(component_names);

	// Add marker
	match (mark_x,mark_y) {
	    (Some(x),Some(y)) => {
		// X = 
		let ixf = ((x - x0)/delta - 0.5).floor();
		let iyf = (ny as f64 - (y - y0)/delta - 0.5).floor();
		if 0.0 <= ixf && 0.0 <= iyf {
		    let ix = ixf as usize;
		    let iy = iyf as usize;
		    if ix < nx && iy < ny {
			println!("Marking ix = {}, iy = {}",ix,iy);
			for ix2 in 0..nx {
			    img[[iy,ix2,0]] ^= 255;
			}
			for iy2 in 0..ny {
			    img[[iy2,ix,0]] ^= 255;
			}
		    } else {
			println!("Not marking, ix = {}, iy = {}",ix,iy);
		    }
		} else {
		    println!("Not marking, ixf = {}, iyf = {}",ixf,iyf);
		}
	    },
	    _ => ()
	}
	    

	ndarray_image::save_image(&format!("layc{}.png",ilay + 1),
				  img.view(),
				  ndarray_image::Colors::Rgb)?;
    }
    
    // Capacitances
    if true {
	for ilay in 0..nlay {
	    println!("Layer {}",ilay);
	    let mut jlays = Vec::new();
	    if ilay > 1 {
		jlays.push(ilay - 1);
	    }
	    if ilay + 1 < nlay {
		jlays.push(ilay + 1);
	    }
	    let cci = &cc[ilay];
	    // TODO: Normalize pairs and sum by nets
	    for jlay in jlays {
		let ccj = &cc[jlay];
		for (icomi,comi) in cci.components.iter().enumerate() {
		    for (icomj,comj) in ccj.components.iter().enumerate() {
			let n = comi.intersection(comj).count();
			if n > 0 {
			    let area = n as f64 * delta * delta * 1e-6;
			    let cap = 8.854e-12 * eps_rel * area
				/ (thickness * 1e-3);
			    if cap >= cap_min {
				println!("{:7.3} pF\t{}[{:02}:{:05}] - {}[{:02}:{:05}]",
					 cap/1e-12,
					 component_names_per_layer[ilay][icomi]
					 .map(|x| x.as_str())
					 .unwrap_or("?"),
					 ilay,icomi,
					 component_names_per_layer[jlay][icomj]
					 .map(|x| x.as_str())
					 .unwrap_or("?"),
					 jlay,icomj);
			    }
			}
		    }
		}
	    }
	    // cc[ilay].dump();
	}
    }
    
    let output_fn : String = args.value_from_str("--output")?;
    let fd = hdf5::File::create(&output_fn)?;
    fd.new_dataset_builder().with_data(&artwork.layers).create("layers")?;
    fd.new_dataset_builder().with_data(&component_ids_per_layer)
	.create("component_ids_per_layer")?;

    Ok(())
}
