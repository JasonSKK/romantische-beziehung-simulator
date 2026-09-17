#![no_std]
#![no_main]
/// This software implements the romantic relationship simulator coding challenge
/// This is an embedded multithreadded application that is meant to run on the re2350 RPI pico2w.
/// The relationship statuses are expresed in LED events on the RP Pico 2 W,
/// each actor BF, GF, OHG connects to GPIO 12, 14, 15 respectively
/// Iason Svoronos - Kanavas 20260917

/// Embassy example desc I used as base for multithreadded blinking -->
/// This example demonstrates how to access a given pin from more than one embassy task
/// The on-board LED is toggled by two tasks with slightly different periods, leading to the
/// apparent duty cycle of the LED increasing, then decreasing, linearly. The phenomenon is similar
/// to interference and the 'beats' you can hear if you play two frequencies close to one another
/// [Link explaining it](https://www.physicsclassroom.com/class/sound/Lesson-3/Interference-and-Beats)

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use panic_probe as _;
use embassy_rp::peripherals::TRNG;
use embassy_rp::trng::Trng;
use embassy_rp::bind_interrupts;

// TRNG INIT 
bind_interrupts!(struct Irqs {
    TRNG_IRQ => embassy_rp::trng::InterruptHandler<TRNG>;
});
static TRNG_LOCK: Mutex<ThreadModeRawMutex, Option<Trng<'static, TRNG>>> = Mutex::new(None);

// LED INIT 
type LedType = Mutex<ThreadModeRawMutex, Option<Output<'static>>>;
static LED_BF: LedType = Mutex::new(None);
static LED_GF: LedType = Mutex::new(None);
static LED_OHG: LedType = Mutex::new(None);

// HEALTH INIT
static HEALTH: Mutex<ThreadModeRawMutex, i8> = Mutex::new(20);

async fn next_random(min: u32, max: u32) -> u32 {
    let mut guard = TRNG_LOCK.lock().await;
    let trng = guard.as_mut().unwrap();
    let mut buf = [0u8; 4];
    trng.fill_bytes(&mut buf).await;
    let raw = u32::from_le_bytes(buf);
    min + (raw % (max - min))
}

async fn flash_led(led: &'static LedType, on_duration: Duration) {
    {
        let mut guard = led.lock().await;
        if let Some(pin) = guard.as_mut() {
            pin.set_low();
        }
    }
    Timer::after(on_duration).await;
    {
        let mut guard = led.lock().await;
        if let Some(pin) = guard.as_mut() {
            pin.set_high();
        }
    }
}

async fn apply_delta(delta: i8, actor: &str, description: &str) {
    let mut health = HEALTH.lock().await;
    *health = (*health + delta).clamp(0, 100);
    defmt::info!("[{}] {} -> health = {}", actor, description, *health);
}

#[embassy_executor::main(executor = "embassy_rp::executor::Executor", entry = "cortex_m_rt::entry")]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // TRNG
    let trng = Trng::new(p.TRNG, Irqs, embassy_rp::trng::Config::default());
    { *(TRNG_LOCK.lock().await) = Some(trng); }

    // blue BF button 
    let button = Input::new(p.PIN_28, Pull::Up);
    
    // set the content of the global LED reference to the real LED pin
    let bf_led = Output::new(p.PIN_12, Level::High);
    let gf_led = Output::new(p.PIN_14, Level::High);
    let ohg_led = Output::new(p.PIN_15, Level::High);
    // inner scope is so that once the mutex is written to, the MutexGuard is dropped, thus the
    // Mutex is released
    {
        *(LED_BF.lock().await) = Some(bf_led);
    }
    {
        *(LED_GF.lock().await) = Some(gf_led);
    }
    {
        *(LED_OHG.lock().await) = Some(ohg_led);
    }

    spawner.spawn(unwrap!(boyfriend_button_task(button)));
    spawner.spawn(unwrap!(girlfriend_task()));
    spawner.spawn(unwrap!(ohg_task()));
    spawner.spawn(unwrap!(monitor_task()));
    
}

// -> BLUE LED
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
        (10, "actually did something nice")
    }
}

async fn boyfriend_event() {
    let event_idx = next_random(0, 3).await;
    let (delta, description) = match event_idx {
        0 => Boyfriend::used_word_optimised_too_often(),
        1 => Boyfriend::explained_something_nobody_asked_about(),
        2 => Boyfriend::actually_did_something_nice(),
        _ => defmt::unreachable!("Boyfriend: bad event index"),
    };
    apply_delta(delta, "Boyfriend", description).await;
    defmt::info!("[Boyfriend Button] {}", description);
    flash_led(&LED_BF, Duration::from_millis(80)).await;
}

#[embassy_executor::task]
async fn boyfriend_button_task(mut button: Input<'static>) {
    loop {
        button.wait_for_falling_edge().await; // blocks here until press proper .await yield no busy loop
        boyfriend_event().await;
    }
}

// -> RED LED
#[derive(Debug)]
struct  Girlfriend { }

impl Girlfriend {

    fn planned_a_nice_evening() -> (i8, &'static str) {
        (6, "planned a nice evening")
    }
    fn gave_the_silent_treatment() -> (i8, &'static str) {
        (-4, "gave the silent treatment")
    }
    fn went_completely_cold_on_nye() -> (i8, &'static str) {
        (-10, "went completely cold on NYE")
    }
}

#[embassy_executor::task]
async fn girlfriend_task() {
    loop {
        let event_idx = next_random(0, 3).await;
        let (delta, description) = match event_idx {
            0 => Girlfriend::planned_a_nice_evening(),
            1 => Girlfriend::gave_the_silent_treatment(),
            2 => Girlfriend::went_completely_cold_on_nye(),
            _ => defmt::unreachable!("Girlfriend: bad event index"),
        };

        apply_delta(delta, "Girlfriend", description).await;
        defmt::info!("[Girlfriend] {}", description);
        flash_led(&LED_GF, Duration::from_millis(150)).await;

        let wait_ms = next_random(200, 2000).await;
        Timer::after(Duration::from_millis(wait_ms as u64)).await;
    }
}

// -> YELLOW LED
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

#[embassy_executor::task]
async fn ohg_task() {
    loop {
        let event_idx = next_random(0, 2).await;
        let (delta, description) = match event_idx {
            0 => OtherHotGuy::liked_a_story(),
            1 => OtherHotGuy::showed_up_at_the_gym(),
            _ => defmt::unreachable!("Girlfriend: bad event index"),
        };

        apply_delta(delta, "Other Hot Guy", description).await;
        defmt::info!("[Other Hot Guy] {}", description);
        flash_led(&LED_OHG, Duration::from_millis(150)).await;

        let wait_ms = next_random(200, 2000).await;
        Timer::after(Duration::from_millis(wait_ms as u64)).await;
    }
}

#[embassy_executor::task]
async fn monitor_task() {
    // let zero_since: Option<Instant> = None;

    loop {
        let current = *HEALTH.lock().await;

        if current <= 0 {
            for _i in 0..20 {
                flash_led(&LED_GF, Duration::from_millis(20)).await;
                Timer::after(Duration::from_millis(40)).await;
            }
            defmt::panic!("Relationship terminated: no recovery in time.");
        }
        Timer::after(Duration::from_millis(100)).await; // dont burn out for no reason
    }
}
