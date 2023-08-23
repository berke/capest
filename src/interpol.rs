use crate::math::*;

pub fn interpol2(a:&Array2<Real>,x:Real,y:Real)->Real {
    let (nx,ny) = a.dim();
    if x < 0.0 || y < 0.0 { return 0.0 };
    let ix = x.floor() as usize;
    let bx = x - real(ix);
    let iy = y.floor() as usize;
    let by = y - real(iy);
    if ix + 1 >= nx || iy + 1 >= ny { return 0.0 };
    let f = |b,x0,x1| x0 + b*(x1-x0);

    let a0 = f(by,a[[ix,iy]] ,a[[ix,iy+1]]);
    let a1 = f(by,a[[ix+1,iy]] ,a[[ix+1,iy+1]]);

    f(bx,a0,a1)
}

// pub fn interpol3(a:ArrayView3::<Real>,Real3([x,y,z]):Real3)->Real {
//     let (nx,ny,nz) = a.dim();
//     if x < 0.0 || y < 0.0 || z < 0.0 { return 0.0 };
//     let ix = x.floor() as usize;
//     let bx = x - real(ix);
//     let iy = y.floor() as usize;
//     let by = y - real(iy);
//     let iz = z.floor() as usize;
//     let bz = z - real(iz);
//     if ix + 1 >= nx || iy + 1 >= ny || iz + 1 >= nz { return 0.0 };
//     let f = |b,x0,x1| x0 + b*(x1-x0);

//     let a00 = f(bz,a[[ix,iy,iz]] ,a[[ix,iy,iz+1]]);
//     let a01 = f(bz,a[[ix,iy+1,iz]] ,a[[ix,iy+1,iz+1]]);
//     let a10 = f(bz,a[[ix+1,iy,iz]] ,a[[ix+1,iy,iz+1]]);
//     let a11 = f(bz,a[[ix+1,iy+1,iz]] ,a[[ix+1,iy+1,iz+1]]);

//     let a0 = f(by,a00,a01);
//     let a1 = f(by,a10,a11);

//     f(bx,a0,a1)
// }
