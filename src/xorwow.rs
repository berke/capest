pub struct Xorwow {
    a:u32,
    b:u32,
    c:u32,
    d:u32,
    counter:u32
}

impl Xorwow {
    pub fn new(seed:u64)->Self {
        let (a,b) = sp(seed);
	let mut s = Xorwow{ a,b,c:1,d:1,counter:0 };
	for _ in 0..1024 {
	    let _ = s.next();
	}
	s
    }

    pub fn reset(&mut self,seed:u32) {
	self.a = seed;
	self.b = 1;
	self.c = 1;
	self.d = 1;
	self.counter = 0;
    }

    pub fn next(&mut self)->u32 {
	let mut t = self.d;
	let s = self.a;
	self.d = self.c;
	self.c = self.b;
	self.b = s;
	t ^= t >> 2;
	t ^= t << 1;
	t ^= s ^ (s << 4);
	self.a = t;
	self.counter = self.counter.wrapping_add(362437);
	let r = t.wrapping_add(self.counter);
	r
    }

    pub fn next64(&mut self)->u64 {
        let a = self.next();
        let b = self.next();
        jn(a,b)
    }

    pub fn rnd(&mut self)->f64 {
	(self.next64() & ((1 << 48) - 1)) as f64 / (1_u64 << 48) as f64
    }
}

fn jn(x:u32,y:u32)->u64 {
    ((x as u64) << 32) | (y as u64)
}

fn sp(x:u64)->(u32,u32) {
    ((x >> 32) as u32, (x & 0xffffffff) as u32)
}
