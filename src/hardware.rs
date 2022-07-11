use sysinfo::{System, SystemExt};

pub const MIN_RAM_KB : u64 = 4294967296;    // 4GB
pub const MIN_CPU_CORES: usize = 2;

pub trait HardwareCheck {
  fn print_hardware_info(sys: &System);
  fn has_enough_resources(sys: &System) -> bool;
}

impl HardwareCheck for System {
  fn print_hardware_info(sys: &System) {
    println!("Available ram: {}", sys.total_memory());
    println!("Available memory: {}", sys.available_memory());
    println!("Available CPU cores: {}", sys.cpus().len());
  }

  fn has_enough_resources(sys: &System) -> bool {
    sys.total_memory()  >= MIN_RAM_KB   &&
    sys.cpus().len()    >= MIN_CPU_CORES
  }
}