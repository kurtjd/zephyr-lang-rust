// Copyright (c) 2026 Open Device Partnership and Contributors
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use embassy_time::Timer;
use static_cell::StaticCell;
use zephyr::{
    device::sensor::{Sensor, SensorValue},
    embassy::Executor,
    printkln,
    raw::{
        sensor_attribute_SENSOR_ATTR_OFFSET as OFFSET,
        sensor_channel_SENSOR_CHAN_AMBIENT_TEMP as AMBIENT_TEMP,
    },
};
use zephyr_panic as _;

#[no_mangle]
extern "C" fn rust_main() {
    printkln!("Rust async sensor sample");

    let sensor = zephyr::devicetree::aliases::sensor0::get_instance()
        .expect("Failed to get sensor0 from devicetree alias");
    assert!(sensor.is_ready(), "Sensor device is not ready");

    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    EXECUTOR.init(Executor::new()).run(|spawner| {
        spawner.spawn(temperature_task(sensor).unwrap());
    });
}

#[embassy_executor::task]
async fn temperature_task(mut sensor: Sensor) {
    // Write then read back an attribute to show the attributes wrapper behaves correctly.
    printkln!("Writing offset...");

    // SAFETY: TMP117 reads one sensor value for the offset attribute.
    unsafe { sensor.attr_set(AMBIENT_TEMP, OFFSET, [SensorValue { val1: 5, val2: 0 }]) }
        .await
        .expect("Failed to set offset");

    // SAFETY: TMP117 writes one sensor value for the offset attribute.
    let [offset] = unsafe { sensor.attr_get::<1>(AMBIENT_TEMP, OFFSET) }
        .await
        .expect("Failed to get offset");

    let offset = sensor_value_to_double(&offset);
    printkln!("Retrieved offset: {:.3} C", offset);

    // Then sample and print the temperature periodically.
    loop {
        // Waiting before fetching also allows the offset to affect a fresh conversion.
        Timer::after_secs(1).await;

        if let Err(error) = sensor.sample_fetch_chan(AMBIENT_TEMP).await {
            printkln!("Temperature fetch failed: {}", error);
            continue;
        }

        let mut values = [SensorValue::default(); 1];

        // SAFETY: TMP117 returns one value for the ambient-temperature channel.
        if let Err(error) = unsafe { sensor.channel_get(AMBIENT_TEMP, &mut values) } {
            printkln!("Temperature read failed: {}", error);
            continue;
        }

        let temperature = sensor_value_to_double(&values[0]);
        printkln!("Temperature: {:.3} C", temperature);
    }
}

// Revisit: Soon safe wrappers around these utility functions will be added so this won't be needed.
fn sensor_value_to_double(value: &SensorValue) -> f64 {
    // SAFETY: The reference points to an initialized sensor value.
    unsafe { zephyr::raw::sensor_value_to_double(value) }
}
