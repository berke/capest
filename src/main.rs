#![allow(dead_code)]
#![allow(unused_imports)]

mod math;
mod config;
mod math_random;
mod interpol;
mod xorwow;
mod spherical;
mod disk;
mod progress;
mod ndarray_image;
mod gerber;
mod common;

use log::{trace,info,error};
use std::collections::{BTreeSet,BTreeMap};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::io::{Read,Write,BufWriter,BufReader};
use math::*;
use pico_args::Arguments;

use xorwow::Xorwow;
use gerber::{Image,NetInfos};
use config::{Config,Loadable};

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
    pub fn new<P:AsRef<Path>>(lay_fns:&[P])->Res<Self> {
	let mut layers_opt = None;
	for (ilay,lay_fn) in lay_fns.iter().enumerate() {
	    // info!("Loading layer {} from {:?}",ilay,lay_fn);
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

struct Registry {
    name_to_id:BTreeMap<String,usize>,
    id_to_name:Vec<String>
}

impl Registry {
    pub fn new()->Self {
	Self {
	    name_to_id:BTreeMap::new(),
	    id_to_name:Vec::new()
	}
    }

    pub fn register(&mut self,name:&str)->usize {
	*self.name_to_id
	    .entry(name.to_string())
	    .or_insert_with(|| {
		let id = self.id_to_name.len();
		self.id_to_name.push(name.to_string());
		id
	    })
    }

    pub fn len(&self)->usize {
	self.id_to_name.len()
    }

    pub fn find_id(&self,name:&str)->Option<usize> {
	self.name_to_id.get(name).copied()
    }

    pub fn find_name(&self,id:usize)->Option<&str> {
	if id < self.id_to_name.len() {
	    Some(&self.id_to_name[id])
	} else {
	    None
	}
    }
}

fn main()->Res<()> {
    simple_logger::SimpleLogger::new().init()?;

    let res = main0();
    if let Err(e) = &res {
	error!("{}",e);
    }

    res
}

fn main0()->Res<()> {
    let mut args = Arguments::from_env();

    let config_fn : String = args.value_from_str("--config")?;
    info!("Loading configuration from {}",config_fn);
    let config = Config::load(&config_fn)?;

    let lay_fns : Vec<String> =
	config.layers
	.iter()
	.map(|l| format!("{}/{}",
			 config.input,
			 l.bitmap))
	.collect();
    let artwork = Artwork::new(&lay_fns)?;
    let (ny,nx) = artwork.layers.dim();
    let nlay = artwork.num_layers;
    info!("Dimensions: {} x {}, number of layers: {}",ny,nx,nlay);

    info!("Creating output directory {}",config.output);
    std::fs::create_dir_all(&config.output)?;

    let mut net_infos = Vec::new();
    for ilay in 0..nlay {
	let path = format!("{}/{}",config.input,config.layers[ilay].gerber);
	let img = Image::from_file(&path)?;
	let infos : NetInfos = (&img).into();
	net_infos.push(infos);
    }

    {
	for ilay in 0..nlay {
	    let lname = &config.layers[ilay].name;
	    let report_path = format!("{}/nets-{}-{}.txt",
				      config.output,
				      ilay,lname);
	    info!("Writing layer {} net report to {}",
		  lname,
		  report_path);
	    let fd = File::create(report_path)?;
	    let mut fd = BufWriter::new(fd);
	    for (name,points) in net_infos[ilay].index.iter() {
		writeln!(fd,
			 "{} {} {} {}",
			 name,
			 points.len(),
			 points[0].x,
			 points[0].y)?;
	    }
	}
    }

    let delta = 25.4 / config.dpi;

    // Origin at bottom-left corner
    // Thus
    //
    // X = delta * (ix + 0.5) + X0
    // Y = (ny - iy - 0.5)*delta + Y0

    // ix = (X - X0)/delta - 0.5
    // iy = ny - (Y - Y0)/delta - 0.5

    info!("Computing connected components");
    let cc = artwork.connected_components();
    let mut component_ids_per_layer = Array3::zeros((nlay,ny,nx));
    let mut component_names_per_layer : Vec<Vec<Option<String>>> = Vec::new();
    let mut xw = Xorwow::new(1);

    info!("Marking components");
    for ilay in 0..nlay {
	let lname = &config.layers[ilay].name;
	let ccs = &cc[ilay];
	let m = ccs.components.len();
	info!("Layer {} ({}), number of components: {}",ilay,lname,m);

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

	let mut component_names : Vec<Option<String>> = vec![None;m];

	let x0 = config.origin.x;
	let y0 = config.origin.y;

	// Try to match components
	info!("Matching components to nets");
	let match_path = format!("{}/net-match-{}-{}.txt",
				  config.output,
				  ilay,lname);
	let fd = File::create(match_path)?;
	let mut fd = BufWriter::new(fd);
	let mut n_out_of_bounds = 0;
	for (name,points) in net_infos[ilay].index.iter() {
	    write!(fd,"{} -> ",name)?;
	    for &gerber::Point { x, y } in points.iter() {
		let ixf = ((x - x0)/delta - 0.5).floor();
		let iyf = (ny as f64 - (y - y0)/delta - 0.5).floor();
		write!(fd,"  {},{} ({},{})",x,y,ixf,iyf)?;
		if 0.0 <= ixf && 0.0 <= iyf {
		    let ix = ixf as usize;
		    let iy = iyf as usize;
		    if ix < nx && iy < ny {
			let icom = component_ids_per_layer[[ilay,iy,ix]];
			if icom > 0 {
			    component_names[icom - 1] = Some(name.clone());
			}
			write!(fd,":{}",icom)?;
		    } else {
			n_out_of_bounds += 1;
			write!(fd,"? (out of bounds, positive)")?;
		    }
		} else {
		    n_out_of_bounds += 1;
		    write!(fd,"? (out of bounds, negative)")?;
		}
	    }
	    writeln!(fd)?;
	}

	if n_out_of_bounds > 0 {
	    error!("Number of components that could not be matched: {}; \
		    check origin and dpi",
		   n_out_of_bounds);
	}

	component_names_per_layer.push(component_names);

	// Add marker
	match config.mark {
	    Some(config::Point{x,y}) => {
		// X = 
		let ixf = ((x - x0)/delta - 0.5).floor();
		let iyf = (ny as f64 - (y - y0)/delta - 0.5).floor();
		if 0.0 <= ixf && 0.0 <= iyf {
		    let ix = ixf as usize;
		    let iy = iyf as usize;
		    if ix < nx && iy < ny {
			info!("Marking ix = {}, iy = {}",ix,iy);
			for ix2 in 0..nx {
			    img[[iy,ix2,0]] ^= 255;
			}
			for iy2 in 0..ny {
			    img[[iy2,ix,0]] ^= 255;
			}
		    } else {
			info!("Not marking, ix = {}, iy = {}",ix,iy);
		    }
		} else {
		    info!("Not marking, ixf = {}, iyf = {}",ixf,iyf);
		}
	    },
	    _ => ()
	}

	ndarray_image::save_image(&format!("{}/layc{}.png",
					   config.output,
					   ilay + 1),
				  img.view(),
				  ndarray_image::Colors::Rgb)?;
    }

    info!("Computing net registry");
    let mut net_names = Registry::new();
    let inc = net_names.register("N/C");
    for ilay in 0..nlay {
	for icomi in 0..cc[ilay].components.len() {
	    if let Some(name) = &component_names_per_layer[ilay][icomi] {
		net_names.register(name);
	    }
	}
    }
    let nnet = net_names.len();
    info!("Total number of unique nets: {}",nnet);

    {
	let nets_path = format!("{}/nets.txt",config.output);
	info!("Writing unique nets to {}",nets_path);
	let fd = File::create(nets_path)?;
	let mut fd = BufWriter::new(fd);
	for (inet,u) in net_names.id_to_name.iter().enumerate() {
	    writeln!(fd,"{} {}",inet,u)?;
	}
    }

    // Capacitances
    info!("Estimating mutual capacitances for adjacent layers");
    let mut caps : BTreeMap<(usize,usize),f64> = BTreeMap::new();
    
    for ilay in 0..nlay {
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
		if let Some(namei) = &component_names_per_layer[ilay][icomi] {
		    let inet = net_names.find_id(namei).unwrap();
		    if inet == inc {
			continue;
		    }
		    for (icomj,comj) in ccj.components.iter().enumerate() {
			if let Some(namej) = &component_names_per_layer[jlay][icomj] {
			    let jnet = net_names.find_id(namej).unwrap();
			    if jnet == inc {
				continue;
			    }
			    if inet != jnet {
				let n = comi.intersection(comj).count();
				if n > 0 {
				    let area = n as f64 * delta * delta * 1e-6;
				    let cap = 8.854e-12 * config.eps_rel * area
					/ (config.thickness * 1e-3);

				    let a = inet.min(jnet);
				    let b = inet.max(jnet);
				    let c = caps.entry((a,b)).or_insert(0.0);
				    *c += cap;

				}
			    }
			}
		    }
		}
	    }
	}
    }

    let mut sig_caps : BTreeMap<i64,(usize,usize)> = BTreeMap::new();
    let scale = 1e-18;
    for (&(inet,jnet),&cap) in caps.iter() {
	if cap >= config.cap_min {
	    let cap_i = (cap/scale).round() as i64;
	    sig_caps.insert(cap_i,(inet,jnet));
	}
    }

    {
	let mutcaps_path = format!("{}/mutcaps.txt",config.output);
	info!("Writing mutual capacitances exceeding {} pF to {}",
	      config.cap_min/1e-12,
	      mutcaps_path);
	let fd = File::create(mutcaps_path)?;
	let mut fd = BufWriter::new(fd);
	for (&cap_i,&(inet,jnet)) in sig_caps.iter() {
	    let cap = cap_i as f64 * (scale/1e-12);
	    writeln!(fd,
		     "{:7.3} pF\t{}\t{}",
		     cap,
		     net_names.find_name(inet).unwrap(),
		     net_names.find_name(jnet).unwrap())?;
	}
    }

    Ok(())
}
