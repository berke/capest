mod math;
mod math_random;
mod interpol;
mod xorwow;
mod spherical;
mod disk;
mod progress;

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

type Res<T> = Result<T,Box<dyn Error>>;

fn error(msg:&str)->Box<dyn Error> {
    Box::new(std::io::Error::new(std::io::ErrorKind::Other,msg))
}

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
    let artwork = Artwork::new(&lay_fns_str)?;
    let (ny,nx) = artwork.layers.dim();
    println!("Dimensions: {} x {}",ny,nx);

    let dpi : f64 = args.opt_value_from_str("--dpi")?.unwrap_or(600.0);
    let eps_rel : f64 = args.opt_value_from_str("--eps-rel")?.unwrap_or(4.2);
    let thickness : f64 = args.opt_value_from_str("--thickness")?.
	unwrap_or(1.6e-3);
    let cap_min : f64 = args.opt_value_from_str("--cap-min")?.unwrap_or(1e-12);

    let cc = artwork.connected_components();
    let nlay = artwork.num_layers;
    
    let delta = 25.4e-3 / dpi;
    
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
	for jlay in jlays {
	    let ccj = &cc[jlay];
	    for (icomi,comi) in cci.components.iter().enumerate() {
		for (icomj,comj) in ccj.components.iter().enumerate() {
		    let n = comi.intersection(comj).count();
		    if n > 0 {
			let area = n as f64 * delta * delta;
			let cap = 8.854e-12 * eps_rel * area / thickness;
			if cap >= cap_min {
			    println!("  {:02}:{:05} - {:02}:{:05} : {:7.3} pF",
				     ilay,icomi,jlay,icomj,
				     cap/1e-12);
			}
		    }
		}
	    }
	}
	// cc[ilay].dump();
    }
    
    let output_fn : String = args.value_from_str("--output")?;
    let fd = hdf5::File::create(&output_fn)?;
    fd.new_dataset_builder().with_data(&artwork.layers).create("layers")?;

    Ok(())
}
