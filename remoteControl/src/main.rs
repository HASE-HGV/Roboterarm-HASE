mod ui;
mod traffic_handler;
mod helpers;

use traffic_handler::init_connection;
use helpers::input;

fn main() {
    println!("please enter IP-address");
    let ip = input();
    
    println!("please enter the mode you wish to use");
    
    let mode_input = input(); 
    
    let mode: u8 = unsafe {
        let ptr = mode_input.as_ptr();
        *ptr
    };

    init_connection(&ip, mode);
}
