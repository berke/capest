use crate::math::*;

#[derive(Debug)]
pub struct DiskIterator {
    i0:isize,
    i1:isize,
    j0:isize,
    j1:isize,
    ic:Real,
    jc:Real,
    r:Real,
    i:isize,
    j:isize
}

impl DiskIterator {
    pub fn new(ic:Real,jc:Real,r:Real,i_min:isize,i_max:isize,
	       j_min:isize,j_max:isize)->Self {
	let i0 = (ic - r).floor() as isize;
	let i1 = (ic + r).ceil() as isize;
	let j0 = (jc - r).floor() as isize;
	let j1 = (jc + r).ceil() as isize;
	let i0 = i0.max(i_min).min(i_max);
	let i1 = i1.max(i_min).min(i_max);
	let j0 = j0.max(j_min).min(j_max);
	let j1 = j1.max(j_min).min(j_max);
	Self{
	    i0,i1,
	    j0,j1,
	    ic,jc,r,
	    i:i0,j:j0 - 1
	}
    }

    pub fn test(&self,i:Real,j:Real)->bool {
	sq(i - self.ic) + sq(j - self.jc) <= sq(self.r)
    }
}

impl Iterator for DiskIterator {
    type Item = (isize,isize);

    fn next(&mut self)->Option<Self::Item> {
	let mut i = self.i;
	let mut j = self.j;
	loop {
	    if j < self.j1 {
		j += 1;
	    } else {
		if i < self.i1 {
		    i += 1;
		    j = 0;
		} else {
		    return None;
		}
	    }

	    let ix = i as Real + 0.5;
	    let jx = j as Real + 0.5;

	    if self.test(ix,jx) {
		self.i = i;
		self.j = j;
		return Some((i,j))
	    }
	}
    }
}
