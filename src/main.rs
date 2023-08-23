mod math;
mod math_random;
mod interpol;
mod xorwow;
mod spherical;
mod disk;
mod progress;

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
	let layers = layers_opt.ok_or_else(|| error("No layers"))?;
	Ok(Self { layers })
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
    
    let output_fn : String = args.value_from_str("--output")?;
    let fd = hdf5::File::create(&output_fn)?;
    fd.new_dataset_builder().with_data(&artwork.layers).create("layers")?;

    Ok(())
}
