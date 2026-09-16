// use std::sync::atomic::{AtomicI8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration};
use chrono::Local;
use log::{Record, Level, Metadata};
use log::{SetLoggerError, LevelFilter};
use std::thread;
use std::process::ExitCode;

static LOGGER: Logger = Logger;

#[derive(Debug)]
pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{}: {} - {}", {Local::now()}, record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info))
}

#[derive(Debug,Clone)]
struct Relationship {
    health: Arc<Mutex<i8>>,       // 0-100, clamp on write
    // last_recovery: Arc<Mutex<Instant>>,
}

impl Relationship {
    fn new() -> Relationship {
        Relationship {
            health: Arc::new(Mutex::new(100)),
            // last_recovery: Arc<Mutex<Instant>>,
            // log: Logger,
        }
    }
    fn apply_delta(&self, delta:i8, actor:&str, description: &str) {
        let mut health = self.health.lock().unwrap();
        *health = (*health + delta).clamp(0, 100);
        // force the result into the [0, 100] range. If health+delta is -5, this returns 0. If it's 130, returns 100. Prevents health going out of bounds.
        // Writes new back into the atomic. Again SeqCst for consistent ordering across threads.
        log::info!("[{actor}] {description} ({delta:+}) -> health = {}", *health);
    }
}

trait Actor {
    fn act(&self, rel: &Relationship);  // called in a loop on its own thread
    // fn led(&self) -> LedChannel;        // which LED reports this actor
}

impl Actor for Boyfriend {
    fn act(&self, rel: &Relationship) {
        let (delta, description) =  match rand::random_range(0..3) { // max range = Boyfriend number of impl fn, as range is exclusive
            0 => Boyfriend::used_word_optimised_too_often(),
            1 => Boyfriend::explained_something_nobody_asked_about(),
            2 => Boyfriend::actually_did_something_nice(),
            _ => unreachable!("Actor for Boyfriend: Incorrect range of state list. Must be 0..3."), //range is 0..3, compiler can't prove exhaustiveness itself
        };
        rel.apply_delta(delta, "Boyfriend", description);
    }

    // fn led(&self) -> LedChannel {
    //     LedChannel::Blue
    // }
}

impl Actor for Girlfriend {
    fn act(&self, rel: &Relationship) {
        let (delta, description) =  match rand::random_range(0..3) { // max range = Girlfriend number of impl fn, as range is exclusive
            0 => Girlfriend::planned_a_nice_evening(),
            1 => Girlfriend::gave_the_silent_treatment(),
            2 => Girlfriend::forgave_something(),
            _ => unreachable!("Actor for Girlfriend: Incorrect range of state list. Must be 0..3."), //range is 0..3, compiler can't prove exhaustiveness itself
        };
        rel.apply_delta(delta, "Girlfriend", description);
    }
    
    // fn led(&self) -> LedChannel {
    //     LedChannel::Blue
    // }
}

impl Actor for OtherHotGuy {
    fn act(&self, rel: &Relationship) {
        let (delta, description) =  match rand::random_range(0..2) { // max range = OtherHotGuy number of impl fn, as range is exclusive
            0 => OtherHotGuy::liked_a_story(),
            1 => OtherHotGuy::showed_up_at_the_gym(),
            _ => unreachable!("Actor for OthoerHotGuy: Incorrect range of state list. Must be 0..2."), //range is 0..2, compiler can't prove exhaustiveness itself
        };
        rel.apply_delta(delta, "OtherHotGuy", description);
    }
    
    // fn led(&self) -> LedChannel {
    //     LedChannel::Blue
    // }
}

// -> Blue LED
#[derive(Debug)]
pub struct Boyfriend { }

impl Boyfriend {
    fn used_word_optimised_too_often() -> (i8, &'static str) {
        (-3, "used the word 'optimized' unprompted")
    }
    fn explained_something_nobody_asked_about() -> (i8, &'static str) {
        (-5, "explained something nobody asked about")
    }
    fn actually_did_something_nice() -> (i8, &'static str) {
        (4, "actually did something nice")
    }
}

// -> Green LED
#[derive(Debug)]
struct  Girlfriend { }

impl Girlfriend {

    fn planned_a_nice_evening() -> (i8, &'static str) {
        (6, "used the word 'optimized' unprompted")
    }
    fn gave_the_silent_treatment() -> (i8, &'static str) {
        (-7, "explained something nobody asked about")
    }
    fn forgave_something() -> (i8, &'static str) {
        (10, "actually did something nice")
    }
}

// -> Red LED
#[derive(Debug)]
struct OtherHotGuy { }

impl OtherHotGuy {

    fn liked_a_story() -> (i8, &'static str) {
        (-2, "liked a story")
    }
    fn showed_up_at_the_gym() -> (i8, &'static str) {
        (-2, "showed up at the gym")
    }

}

fn main() -> ExitCode {

    init().unwrap(); // init logger
    let rel = Relationship::new();
    
    while *rel.health.lock().unwrap() != 0 {
        // let random_gen: u64  = rand::random_range(200..2000); // does not update for now
        let bf_random_gen: u64  = rand::random_range(200..2000);
        let gf_random_gen: u64  = rand::random_range(200..2000);
        let ohg_random_gen: u64  = rand::random_range(200..2000); 

        let bf = Boyfriend {};
        let gf = Girlfriend {};
        let ohg = OtherHotGuy {};

        // Boyfried thread applies at random interval health delta
        let bf_rel_clone = rel.clone();
        let bf_handle = thread::spawn(move || {
            /*  Three distinct Arc handles (bf_handle, gf_handle, ohg_handle), three distinct variables to move (bf_rel_clone, gf_rel_clone, ohg_rel_clone) 
            but all three still point at the exact same Relationship.health Mutex<i8> in memory (that is the whole point of Arc: many pointers, one shared allocation). */ 
            thread::sleep(Duration::from_millis(bf_random_gen));
            bf.act(&bf_rel_clone);
        });
        
        // Girlfriend thread applies at random interval health delta
        let gf_rel_clone = rel.clone();
        let gf_handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(gf_random_gen));
            gf.act(&gf_rel_clone);
        });
        
        // OtherHotGuy thread applies at random interval health delta
        let ohg_rel_clone = rel.clone();
        let ohg_handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(ohg_random_gen));
            ohg.act(&ohg_rel_clone);
        });

        bf_handle.join().unwrap(); // capture the JoinHandle and .join() it so main waits:
        gf_handle.join().unwrap();
        ohg_handle.join().unwrap();

    }

    log::error!("Oh no! GF went suddenly cold after meeting OtherHotGuy, broke up with you, and you end up with trauma");

    ExitCode::SUCCESS

}
