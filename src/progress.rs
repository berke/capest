use crate::math::*;

pub struct ProgressIndicator {
    total: usize,
    current: usize,
    last: usize,
    rate: Real,
    t_first: Real,
    t_prev: Real,
    t_last: Real,
    delta_t: Real,
    label: String
}

fn now()->f64 {
    let dt = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    (dt.as_secs() as f64) + 1e-9*(dt.subsec_nanos() as f64)
}

impl ProgressIndicator {
    pub fn new(lbl:&str,total:usize)->Self {
        ProgressIndicator {
            total,
            current: 0,
            last: 0,
            rate: 0.0,
            t_first: now(),
            t_prev: 0.0,
            t_last: 0.0,
            delta_t: 0.5,
            label: lbl.to_string()
        }
    }
    pub fn set_label(&mut self,lbl:&str) {
	self.label = lbl.to_string();
    }
    pub fn set(&mut self,total:usize) {
	self.total = total;
	self.current = 0;
	self.last = 0;
	self.rate = 0.0;
    }
    pub fn update(&mut self,current:usize) {
        self.current = current;
        if real(current) >= real(self.last) + self.rate*self.delta_t {
            let t = now();
            if t > self.t_prev + self.delta_t {
                let new_rate = real(current - self.last) / (t - self.t_last);
                self.last = current;
                self.rate = max(1.0, (2.0*self.rate + new_rate)/3.0);
                self.t_last = t;
                self.t_prev = t;
                self.display();
            }
        }
    }
    pub fn display(&self) {
        let elp = self.t_last - self.t_first;
        let eta = real(self.total - self.current) / (real(self.current)/(self.t_last - self.t_first));
        println!("{:20} {:12} {:6.2}% elp {:8.1} ETA {:8.1} est {:8.1}",
                 self.label, self.current, 100.0*real(self.current)/real(self.total),
                 elp, eta, elp+eta);
    }
}
