use wgpu_sky_rendering::run;
fn main() {
    pollster::block_on(run());
}
