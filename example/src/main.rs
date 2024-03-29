use std::{marker::PhantomData, time::Instant};

use ecs::MyEcsBuilder;
use secs::{
    join::Join,
    storage::{Read, Write},
};

pub mod ecs;

fn main() {
    let mut ecs = MyEcsBuilder::new()
        .resource_delta_time(DeltaTime(1e-3))
        .with_capacity(10000);

    println!("ECS initialized");

    for _ in 0..1000 {
        ecs.build(
            ecs.next()
                .acceleration(Acceleration {
                    x: 0.0,
                    y: -9.81,
                    z: 0.0,
                })
                .position(Position {
                    x: 0.0,
                    y: 50.0,
                    z: 0.0,
                    _phantom: Default::default(),
                })
                .velocity(Velocity {
                    x: 50.0,
                    y: 0.0,
                    z: 15.5,
                }),
        );
    }

    for _ in 0..9000 {
        ecs.build(ecs.next().position(Position {
            x: 0.0,
            y: -9.81,
            z: 0.0,
            _phantom: Default::default(),
        }));
    }

    println!("Data generated");

    for _ in 0..1000000 {
        ecs.run().unwrap();
    }

    let start = Instant::now();

    for _ in 0..1000000 {
        ecs.run().unwrap();
    }

    let elapsed = start.elapsed();

    println!(
        "Ran in {:?}, so {:.3} µs/iter, so {:.3} ns/entity",
        elapsed,
        elapsed.as_micros() as f64 / 1000000.0,
        elapsed.as_nanos() as f64 / 1000000000.0
    );
}

#[derive(Clone, Debug, Copy, Default)]
pub struct DeltaTime(f32);

#[derive(Clone, Debug, Copy)]
pub struct Acceleration {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Debug, Copy)]
pub struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Debug, Copy)]
pub struct Position<'a> {
    x: f32,
    y: f32,
    z: f32,
    _phantom: PhantomData<&'a ()>,
}

#[derive(Clone, Debug, Copy, Default)]
pub struct Enabled;

pub fn physics_system<'a>(pos: &mut Position<'a>, velo: &Velocity) {
    pos.x += velo.x;
    pos.y += velo.y;
    pos.z += velo.z;
}

pub fn test_system(delta_time: &mut DeltaTime) {
    delta_time.0 = 1e-3;
}

pub fn second_system<'sys, 'a>(
    mut pos: Write<'sys, Position<'a>, "position">,
    velo: Read<'sys, Velocity, "velocity">,
) {
    for (pos, velo) in (&mut pos, &velo).join() {
        pos.x += velo.x;
        pos.y += velo.y;
        pos.z += velo.z;
    }
}
