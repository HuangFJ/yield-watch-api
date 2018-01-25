#![feature(bench)]

extern crate bus;

use bus::Bus;

fn main(){
    let mut bus = Bus::new(10);
    let mut rx1 = bus.add_rx();
    let mut rx2 = bus.add_rx();

    bus.broadcast("Hello");
    assert_eq!(rx1.recv(), Ok("Hello"));
    assert_eq!(rx2.recv(), Ok("Hello"));
}


