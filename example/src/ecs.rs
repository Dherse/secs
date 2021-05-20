#![allow(unused_variables)]
pub struct MyEcs {
    components: MyEcsComponentStore,
    resource_delta_time: crate::DeltaTime,
}
impl MyEcs {
    #[doc = "Creates a builder for this ECS"]
    pub fn builder() -> MyEcsBuilder {
        MyEcsBuilder::new()
    }
    #[doc = "Runs the ECS"]
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let components = &mut self.components;
        for id in ::secs::hibitset::BitSetAnd(
            ::secs::hibitset::BitSetAnd(&components.bitset_velocity, &components.bitset_position),
            &components.alive,
        ) {
            let id = ::secs::Entity::new(id);
            let sys_physics_comp_position = components
                .position
                .get_mut(id.index() as usize)
                .unwrap()
                .as_mut()
                .unwrap();
            let sys_physics_comp_velocity = components
                .velocity
                .get(id.index() as usize)
                .unwrap()
                .as_ref()
                .unwrap();
            crate::physics_system(sys_physics_comp_position, sys_physics_comp_velocity);
        }
        Ok(())
    }
    #[doc = "Returns a new entity builder"]
    pub fn next(&self) -> MyEcsEntityBuilder {
        MyEcsEntityBuilder::new(self.components.next())
    }
    pub fn build(&mut self, builder: MyEcsEntityBuilder) {
        self.components.build(builder);
    }
    #[doc = "Gets an immutable reference to the component store"]
    pub fn components(&self) -> &MyEcsComponentStore {
        &self.components
    }
    #[doc = "Gets a mutable reference to the component store"]
    pub fn components_mut(&mut self) -> &mut MyEcsComponentStore {
        &mut self.components
    }
}
#[derive(Default)]
pub struct MyEcsBuilder {
    resource_delta_time: crate::DeltaTime,
}
impl MyEcsBuilder {
    #[doc = "Creates a new builder"]
    pub fn new() -> Self {
        Self::default()
    }
    #[doc = "Builds the builder into the ECS"]
    pub fn build(self) -> MyEcs {
        MyEcs {
            components: MyEcsComponentStore::new(),
            resource_delta_time: self.resource_delta_time,
        }
    }
    #[doc = "Builds the builder into the ECS with a capacity"]
    pub fn with_capacity(self, capacity: usize) -> MyEcs {
        MyEcs {
            components: MyEcsComponentStore::with_capacity(capacity),
            resource_delta_time: self.resource_delta_time,
        }
    }
    #[doc = "Sets the resource 'delta_time' of type [`crate::DeltaTime`]"]
    pub fn resource_delta_time(mut self, value: crate::DeltaTime) -> Self {
        self.resource_delta_time = value;
        self
    }
}
pub struct MyEcsComponentStore {
    max: ::std::sync::atomic::AtomicU32,
    freed_rx: std::sync::mpsc::Receiver<u32>,
    freed_tx: std::sync::mpsc::Sender<u32>,
    alive: ::secs::hibitset::BitSet,
    position: Vec<Option<crate::Position>>,
    velocity: Vec<Option<crate::Velocity>>,
    acceleration: Vec<Option<crate::Acceleration>>,
    enabled: (),
    bitset_position: ::secs::hibitset::BitSet,
    bitset_velocity: ::secs::hibitset::BitSet,
    bitset_acceleration: ::secs::hibitset::BitSet,
    bitset_enabled: ::secs::hibitset::BitSet,
}
impl MyEcsComponentStore {
    #[doc = "Initializes a new component store"]
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            max: ::std::sync::atomic::AtomicU32::new(0),
            alive: ::secs::hibitset::BitSet::new(),
            freed_rx: rx,
            freed_tx: tx,
            position: Vec::new(),
            velocity: Vec::new(),
            acceleration: Vec::new(),
            enabled: (),
            bitset_position: ::secs::hibitset::BitSet::new(),
            bitset_velocity: ::secs::hibitset::BitSet::new(),
            bitset_acceleration: ::secs::hibitset::BitSet::new(),
            bitset_enabled: ::secs::hibitset::BitSet::new(),
        }
    }
    #[doc = "Initializes a new component store with a base capacity"]
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            max: ::std::sync::atomic::AtomicU32::new(0),
            alive: ::secs::hibitset::BitSet::new(),
            freed_rx: rx,
            freed_tx: tx,
            position: Vec::with_capacity(capacity),
            velocity: Vec::with_capacity(capacity),
            acceleration: Vec::with_capacity(capacity),
            enabled: (),
            bitset_position: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
            bitset_velocity: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
            bitset_acceleration: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
            bitset_enabled: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
        }
    }
    #[doc = "Checks if an `entity` is alive"]
    pub fn is_alive(&self, entity: ::secs::Entity) -> bool {
        self.alive.contains(entity.index())
    }
    #[doc = "Reserves an entity id, this entity is dead until it has been built!"]
    pub fn next(&self) -> ::secs::Entity {
        if let Ok(value) = self.freed_rx.try_recv() {
            ::secs::Entity::new(value)
        } else {
            ::secs::Entity::new(self.max.fetch_add(1, ::std::sync::atomic::Ordering::SeqCst))
        }
    }
    #[doc = "Adds an entity (dead or alive) to the list of alive entities and clears all of its components"]
    pub fn reset(&mut self, entity: ::secs::Entity) {
        self.alive.add(entity.index());
        if (entity.index() as usize) <= self.position.len() {
            self.position[entity.index() as usize] = None;
            self.bitset_position.remove(entity.index());
        } else {
            self.position.resize(entity.index() as usize + 1, None)
        }
        if (entity.index() as usize) <= self.velocity.len() {
            self.velocity[entity.index() as usize] = None;
            self.bitset_velocity.remove(entity.index());
        } else {
            self.velocity.resize(entity.index() as usize + 1, None)
        }
        if (entity.index() as usize) <= self.acceleration.len() {
            self.acceleration[entity.index() as usize] = None;
            self.bitset_acceleration.remove(entity.index());
        } else {
            self.acceleration.resize(entity.index() as usize + 1, None)
        }
        self.bitset_enabled.remove(entity.index());
    }
    pub fn build(&mut self, builder: MyEcsEntityBuilder) {
        self.alive.add(builder.entity.index());
        if let Some(value) = builder.position {
            self.bitset_position.add(builder.entity.index());
            if self.position.len() <= builder.entity.index() as usize {
                self.position
                    .resize(builder.entity.index() as usize + 1, None);
            }
            self.position[builder.entity.index() as usize] = Some(value);
        } else {
            let exists = self.bitset_position.remove(builder.entity.index());
            if exists {
                self.position[builder.entity.index() as usize].take();
            }
        }
        if let Some(value) = builder.velocity {
            self.bitset_velocity.add(builder.entity.index());
            if self.velocity.len() <= builder.entity.index() as usize {
                self.velocity
                    .resize(builder.entity.index() as usize + 1, None);
            }
            self.velocity[builder.entity.index() as usize] = Some(value);
        } else {
            let exists = self.bitset_velocity.remove(builder.entity.index());
            if exists {
                self.velocity[builder.entity.index() as usize].take();
            }
        }
        if let Some(value) = builder.acceleration {
            self.bitset_acceleration.add(builder.entity.index());
            if self.acceleration.len() <= builder.entity.index() as usize {
                self.acceleration
                    .resize(builder.entity.index() as usize + 1, None);
            }
            self.acceleration[builder.entity.index() as usize] = Some(value);
        } else {
            let exists = self.bitset_acceleration.remove(builder.entity.index());
            if exists {
                self.acceleration[builder.entity.index() as usize].take();
            }
        }
        if let Some(value) = builder.enabled {
            self.bitset_enabled.add(builder.entity.index());
        } else {
            let exists = self.bitset_enabled.remove(builder.entity.index());
            if exists {
                self.enabled;
                if exists {
                    Some(crate::Enabled::default())
                } else {
                    None
                };
            }
        }
    }
    #[doc = "Kills an entity, returns true if the entity was alive"]
    pub fn kill(&mut self, entity: ::secs::Entity) -> bool {
        if self.alive.remove(entity.index()) {
            self.freed_tx
                .send(entity.index())
                .expect("Failed to queue ID reuse");
            {
                let exists = self.bitset_position.remove(entity.index());
                self.position[entity.index() as usize].take();
            }
            {
                let exists = self.bitset_velocity.remove(entity.index());
                self.velocity[entity.index() as usize].take();
            }
            {
                let exists = self.bitset_acceleration.remove(entity.index());
                self.acceleration[entity.index() as usize].take();
            }
            {
                let exists = self.bitset_enabled.remove(entity.index());
                self.enabled;
                if exists {
                    Some(crate::Enabled::default())
                } else {
                    None
                };
            }
            true
        } else {
            false
        }
    }
    #[doc = "Gets a reference to the component 'position' of type [`crate::Position`] from the `entity` if it exists"]
    pub fn position(&self, entity: ::secs::Entity) -> Option<&crate::Position> {
        if !self.alive.contains(entity.index()) || !self.bitset_position.contains(entity.index()) {
            return None;
        }
        self.position.get(entity.index() as usize).unwrap().as_ref()
    }
    #[doc = "Gets a mutable reference to the component 'position' of type [`crate::Position`] from the `entity` if it exists"]
    pub fn position_mut(&mut self, entity: ::secs::Entity) -> Option<&mut crate::Position> {
        if !self.alive.contains(entity.index()) || !self.bitset_position.contains(entity.index()) {
            return None;
        }
        self.position
            .get_mut(entity.index() as usize)
            .unwrap()
            .as_mut()
    }
    #[doc = "Adds the component 'position' of type [`crate::Position`] to the `entity`"]
    pub fn add_position(&mut self, entity: ::secs::Entity, value: crate::Position) -> &mut Self {
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        self.bitset_position.add(entity.index());
        if self.position.len() <= entity.index() as usize {
            self.position.resize(entity.index() as usize + 1, None);
        }
        self.position[entity.index() as usize] = Some(value);
        self
    }
    #[doc = "Removes the component 'position' of type [`crate::Position`] from the `entity`, returns the component if it had it"]
    pub fn del_position(&mut self, entity: ::secs::Entity) -> Option<crate::Position> {
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        let exists = self.bitset_position.remove(entity.index());
        if exists {
            self.position[entity.index() as usize].take()
        } else {
            None
        }
    }
    #[doc = "Gets a reference to the component 'velocity' of type [`crate::Velocity`] from the `entity` if it exists"]
    pub fn velocity(&self, entity: ::secs::Entity) -> Option<&crate::Velocity> {
        if !self.alive.contains(entity.index()) || !self.bitset_velocity.contains(entity.index()) {
            return None;
        }
        self.velocity.get(entity.index() as usize).unwrap().as_ref()
    }
    #[doc = "Gets a mutable reference to the component 'velocity' of type [`crate::Velocity`] from the `entity` if it exists"]
    pub fn velocity_mut(&mut self, entity: ::secs::Entity) -> Option<&mut crate::Velocity> {
        if !self.alive.contains(entity.index()) || !self.bitset_velocity.contains(entity.index()) {
            return None;
        }
        self.velocity
            .get_mut(entity.index() as usize)
            .unwrap()
            .as_mut()
    }
    #[doc = "Adds the component 'velocity' of type [`crate::Velocity`] to the `entity`"]
    pub fn add_velocity(&mut self, entity: ::secs::Entity, value: crate::Velocity) -> &mut Self {
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        self.bitset_velocity.add(entity.index());
        if self.velocity.len() <= entity.index() as usize {
            self.velocity.resize(entity.index() as usize + 1, None);
        }
        self.velocity[entity.index() as usize] = Some(value);
        self
    }
    #[doc = "Removes the component 'velocity' of type [`crate::Velocity`] from the `entity`, returns the component if it had it"]
    pub fn del_velocity(&mut self, entity: ::secs::Entity) -> Option<crate::Velocity> {
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        let exists = self.bitset_velocity.remove(entity.index());
        if exists {
            self.velocity[entity.index() as usize].take()
        } else {
            None
        }
    }
    #[doc = "Gets a reference to the component 'acceleration' of type [`crate::Acceleration`] from the `entity` if it exists"]
    pub fn acceleration(&self, entity: ::secs::Entity) -> Option<&crate::Acceleration> {
        if !self.alive.contains(entity.index())
            || !self.bitset_acceleration.contains(entity.index())
        {
            return None;
        }
        self.acceleration
            .get(entity.index() as usize)
            .unwrap()
            .as_ref()
    }
    #[doc = "Gets a mutable reference to the component 'acceleration' of type [`crate::Acceleration`] from the `entity` if it exists"]
    pub fn acceleration_mut(&mut self, entity: ::secs::Entity) -> Option<&mut crate::Acceleration> {
        if !self.alive.contains(entity.index())
            || !self.bitset_acceleration.contains(entity.index())
        {
            return None;
        }
        self.acceleration
            .get_mut(entity.index() as usize)
            .unwrap()
            .as_mut()
    }
    #[doc = "Adds the component 'acceleration' of type [`crate::Acceleration`] to the `entity`"]
    pub fn add_acceleration(
        &mut self,
        entity: ::secs::Entity,
        value: crate::Acceleration,
    ) -> &mut Self {
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        self.bitset_acceleration.add(entity.index());
        if self.acceleration.len() <= entity.index() as usize {
            self.acceleration.resize(entity.index() as usize + 1, None);
        }
        self.acceleration[entity.index() as usize] = Some(value);
        self
    }
    #[doc = "Removes the component 'acceleration' of type [`crate::Acceleration`] from the `entity`, returns the component if it had it"]
    pub fn del_acceleration(&mut self, entity: ::secs::Entity) -> Option<crate::Acceleration> {
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        let exists = self.bitset_acceleration.remove(entity.index());
        if exists {
            self.acceleration[entity.index() as usize].take()
        } else {
            None
        }
    }
    #[doc = "Checks whether the `entity` has component 'enabled' of type [`crate::Enabled`]"]
    pub fn enabled(&self, entity: ::secs::Entity) -> bool {
        self.alive.contains(entity.index()) && self.bitset_enabled.contains(entity.index())
    }
    #[doc = "Adds the component 'enabled' of type [`crate::Enabled`] to the `entity`"]
    pub fn add_enabled(&mut self, entity: ::secs::Entity, value: crate::Enabled) -> &mut Self {
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        self.bitset_enabled.add(entity.index());
        self
    }
    #[doc = "Removes the component 'enabled' of type [`crate::Enabled`] from the `entity`, returns the component if it had it"]
    pub fn del_enabled(&mut self, entity: ::secs::Entity) -> Option<crate::Enabled> {
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        let exists = self.bitset_enabled.remove(entity.index());
        if exists {
            self.enabled;
            if exists {
                Some(crate::Enabled::default())
            } else {
                None
            }
        } else {
            None
        }
    }
}
pub struct MyEcsEntityBuilder {
    entity: ::secs::Entity,
    position: Option<crate::Position>,
    velocity: Option<crate::Velocity>,
    acceleration: Option<crate::Acceleration>,
    enabled: Option<crate::Enabled>,
}
impl MyEcsEntityBuilder {
    fn new(entity: ::secs::Entity) -> Self {
        Self {
            entity,
            position: None,
            velocity: None,
            acceleration: None,
            enabled: None,
        }
    }
    pub fn entity(&self) -> ::secs::Entity {
        self.entity
    }
    #[doc = "Adds the component 'position' of type [`crate::Position`] to the entity"]
    pub fn position(mut self, value: crate::Position) -> Self {
        self.position = Some(value);
        self
    }
    #[doc = "Adds the component 'position' of type [`crate::Position`] to the entity"]
    pub fn add_position(&mut self, value: crate::Position) -> &mut Self {
        self.position = Some(value);
        self
    }
    #[doc = "Removes the component 'position' of type [`crate::Position`] to the entity"]
    pub fn del_position(&mut self) -> &mut Self {
        self.position = None;
        self
    }
    #[doc = "Adds the component 'velocity' of type [`crate::Velocity`] to the entity"]
    pub fn velocity(mut self, value: crate::Velocity) -> Self {
        self.velocity = Some(value);
        self
    }
    #[doc = "Adds the component 'velocity' of type [`crate::Velocity`] to the entity"]
    pub fn add_velocity(&mut self, value: crate::Velocity) -> &mut Self {
        self.velocity = Some(value);
        self
    }
    #[doc = "Removes the component 'velocity' of type [`crate::Velocity`] to the entity"]
    pub fn del_velocity(&mut self) -> &mut Self {
        self.velocity = None;
        self
    }
    #[doc = "Adds the component 'acceleration' of type [`crate::Acceleration`] to the entity"]
    pub fn acceleration(mut self, value: crate::Acceleration) -> Self {
        self.acceleration = Some(value);
        self
    }
    #[doc = "Adds the component 'acceleration' of type [`crate::Acceleration`] to the entity"]
    pub fn add_acceleration(&mut self, value: crate::Acceleration) -> &mut Self {
        self.acceleration = Some(value);
        self
    }
    #[doc = "Removes the component 'acceleration' of type [`crate::Acceleration`] to the entity"]
    pub fn del_acceleration(&mut self) -> &mut Self {
        self.acceleration = None;
        self
    }
    #[doc = "Adds the component 'enabled' of type [`crate::Enabled`] to the entity"]
    pub fn enabled(mut self, value: crate::Enabled) -> Self {
        self.enabled = Some(value);
        self
    }
    #[doc = "Adds the component 'enabled' of type [`crate::Enabled`] to the entity"]
    pub fn add_enabled(&mut self, value: crate::Enabled) -> &mut Self {
        self.enabled = Some(value);
        self
    }
    #[doc = "Removes the component 'enabled' of type [`crate::Enabled`] to the entity"]
    pub fn del_enabled(&mut self) -> &mut Self {
        self.enabled = None;
        self
    }
}
