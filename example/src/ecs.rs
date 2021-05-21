#![allow(unused_variables, dead_code)]
pub struct MyEcs<'position> {
    components: MyEcsComponentStore<'position>,
    command_buffer: MyEcsCommandBuffer<'position>,
    resource_delta_time: crate::DeltaTime,
}
impl<'position> MyEcs<'position> {
    #[doc = "Creates a builder for this ECS"]
    pub fn builder() -> MyEcsBuilder {
        MyEcsBuilder::new()
    }
    #[doc = "Runs the ECS"]
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let components = &mut self.components;
        for id in
            ::secs::hibitset::BitSetAnd(&components.bitset_velocity, &components.bitset_position)
        {
            let id = ::secs::Entity::new(id);
            let entt = id;
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
            crate::physics_system(entt, sys_physics_comp_position, sys_physics_comp_velocity);
        }
        self.command_buffer.build(&mut self.components);
        Ok(())
    }
    #[doc = "Returns a new entity builder"]
    pub fn next(&self) -> MyEcsEntityBuilder<'position> {
        <MyEcsEntityBuilder>::new(self.components.next())
    }
    pub fn build(&mut self, builder: MyEcsEntityBuilder<'position>) {
        self.components.build(builder);
    }
    #[doc = "Gets an immutable reference to the component store"]
    pub fn components(&self) -> &MyEcsComponentStore<'position> {
        &self.components
    }
    #[doc = "Gets a mutable reference to the component store"]
    pub fn components_mut(&mut self) -> &mut MyEcsComponentStore<'position> {
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
    pub fn build<'position>(self) -> MyEcs<'position> {
        let components = MyEcsComponentStore::new();
        MyEcs {
            command_buffer: MyEcsCommandBuffer::new(&components),
            components,
            resource_delta_time: self.resource_delta_time,
        }
    }
    #[doc = "Builds the builder into the ECS with a capacity"]
    pub fn with_capacity<'position>(self, capacity: usize) -> MyEcs<'position> {
        let components = MyEcsComponentStore::with_capacity(capacity);
        MyEcs {
            command_buffer: MyEcsCommandBuffer::new(&components),
            components,
            resource_delta_time: self.resource_delta_time,
        }
    }
    #[doc = "Sets the resource 'delta_time' of type [`crate::DeltaTime`]"]
    pub fn resource_delta_time(mut self, value: crate::DeltaTime) -> Self {
        self.resource_delta_time = value;
        self
    }
}
pub struct MyEcsComponentStore<'position> {
    max: ::std::sync::Arc<::std::sync::atomic::AtomicU32>,
    freed_rx: ::secs::crossbeam_channel::Receiver<u32>,
    freed_tx: ::secs::crossbeam_channel::Sender<u32>,
    alive: ::secs::hibitset::BitSet,
    position: Vec<Option<crate::Position<'position>>>,
    velocity: Vec<Option<crate::Velocity>>,
    acceleration: Vec<Option<crate::Acceleration>>,
    enabled: (),
    bitset_position: ::secs::hibitset::BitSet,
    bitset_velocity: ::secs::hibitset::BitSet,
    bitset_acceleration: ::secs::hibitset::BitSet,
    bitset_enabled: ::secs::hibitset::BitSet,
}
impl<'position> Default for MyEcsComponentStore<'position> {
    fn default() -> Self {
        Self::new()
    }
}
impl<'position> MyEcsComponentStore<'position> {
    #[doc = "Initializes a new component store"]
    pub fn new() -> Self {
        let (tx, rx) = ::secs::crossbeam_channel::unbounded();
        Self {
            max: ::std::sync::Arc::new(::std::sync::atomic::AtomicU32::new(0)),
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
        let (tx, rx) = ::secs::crossbeam_channel::unbounded();
        Self {
            max: ::std::sync::Arc::new(::std::sync::atomic::AtomicU32::new(0)),
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
    pub fn alive(&self, entity: ::secs::Entity) -> bool {
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
    pub fn build(&mut self, builder: MyEcsEntityBuilder<'position>) {
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
                if exists {
                    Some(<crate::Enabled>::default())
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
                if exists {
                    Some(<crate::Enabled>::default())
                } else {
                    None
                };
            }
            true
        } else {
            false
        }
    }
    #[doc = "Gets a reference to the component 'position' of type [`crate::Position<'position>`] from the `entity` if it exists"]
    pub fn position(&self, entity: ::secs::Entity) -> Option<&crate::Position<'position>> {
        if !self.alive.contains(entity.index()) || !self.bitset_position.contains(entity.index()) {
            return None;
        }
        self.position.get(entity.index() as usize).unwrap().as_ref()
    }
    #[doc = "Gets a mutable reference to the component 'position' of type [`crate::Position<'position>`] from the `entity` if it exists"]
    pub fn position_mut(
        &mut self,
        entity: ::secs::Entity,
    ) -> Option<&mut crate::Position<'position>> {
        if !self.alive.contains(entity.index()) || !self.bitset_position.contains(entity.index()) {
            return None;
        }
        self.position
            .get_mut(entity.index() as usize)
            .unwrap()
            .as_mut()
    }
    #[doc = "Adds the component 'position' of type [`crate::Position<'position>`] to the `entity`"]
    pub fn add_position(
        &mut self,
        entity: ::secs::Entity,
        value: crate::Position<'position>,
    ) -> &mut Self {
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        self.bitset_position.add(entity.index());
        if self.position.len() <= entity.index() as usize {
            self.position.resize(entity.index() as usize + 1, None);
        }
        self.position[entity.index() as usize] = Some(value);
        self
    }
    #[doc = "Removes the component 'position' of type [`crate::Position<'position>`] from the `entity`, returns the component if it had it"]
    pub fn del_position(&mut self, entity: ::secs::Entity) -> Option<crate::Position<'position>> {
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
            if exists {
                Some(<crate::Enabled>::default())
            } else {
                None
            }
        } else {
            None
        }
    }
}
pub struct MyEcsEntityBuilder<'position> {
    entity: ::secs::Entity,
    position: Option<crate::Position<'position>>,
    velocity: Option<crate::Velocity>,
    acceleration: Option<crate::Acceleration>,
    enabled: Option<crate::Enabled>,
}
impl<'position> MyEcsEntityBuilder<'position> {
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
    #[doc = "Adds the component 'position' of type [`crate::Position<'position>`] to the entity"]
    pub fn position(mut self, value: crate::Position<'position>) -> Self {
        self.position = Some(value);
        self
    }
    #[doc = "Adds the component 'position' of type [`crate::Position<'position>`] to the entity"]
    pub fn add_position(&mut self, value: crate::Position<'position>) -> &mut Self {
        self.position = Some(value);
        self
    }
    #[doc = "Removes the component 'position' of type [`crate::Position<'position>`] to the entity"]
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
pub struct MyEcsCommandBuffer<'position> {
    next: ::std::sync::Arc<::std::sync::atomic::AtomicU32>,
    receiver: ::secs::crossbeam_channel::Receiver<u32>,
    new_entities: Vec<MyEcsEntityBuilder<'position>>,
    deleted_entities: ::secs::fxhash::FxHashSet<::secs::Entity>,
    add_position: ::secs::fxhash::FxHashMap<::secs::Entity, crate::Position<'position>>,
    del_position: ::secs::fxhash::FxHashSet<::secs::Entity>,
    add_velocity: ::secs::fxhash::FxHashMap<::secs::Entity, crate::Velocity>,
    del_velocity: ::secs::fxhash::FxHashSet<::secs::Entity>,
    add_acceleration: ::secs::fxhash::FxHashMap<::secs::Entity, crate::Acceleration>,
    del_acceleration: ::secs::fxhash::FxHashSet<::secs::Entity>,
    add_enabled: ::secs::fxhash::FxHashMap<::secs::Entity, crate::Enabled>,
    del_enabled: ::secs::fxhash::FxHashSet<::secs::Entity>,
}
impl<'position> MyEcsCommandBuffer<'position> {
    #[doc = "Creates a new command buffer"]
    fn new(store: &MyEcsComponentStore<'position>) -> Self {
        Self {
            new_entities: Vec::new(),
            next: ::std::sync::Arc::clone(&store.max),
            receiver: store.freed_rx.clone(),
            deleted_entities: ::secs::fxhash::FxHashSet::default(),
            add_position: ::secs::fxhash::FxHashMap::default(),
            del_position: ::secs::fxhash::FxHashSet::default(),
            add_velocity: ::secs::fxhash::FxHashMap::default(),
            del_velocity: ::secs::fxhash::FxHashSet::default(),
            add_acceleration: ::secs::fxhash::FxHashMap::default(),
            del_acceleration: ::secs::fxhash::FxHashSet::default(),
            add_enabled: ::secs::fxhash::FxHashMap::default(),
            del_enabled: ::secs::fxhash::FxHashSet::default(),
        }
    }
    #[doc = "Schedules the creation of an entity, already reserving its ID"]
    pub fn entity<F: Fn(::secs::Entity, &mut MyEcsEntityBuilder<'position>)>(
        &mut self,
        fun: F,
    ) -> ::secs::Entity {
        let entity = if let Ok(value) = self.receiver.try_recv() {
            ::secs::Entity::new(value)
        } else {
            ::secs::Entity::new(
                self.next
                    .fetch_add(1, ::std::sync::atomic::Ordering::SeqCst),
            )
        };
        let mut entity_builder = MyEcsEntityBuilder::new(entity);
        fun(entity, &mut entity_builder);
        self.new_entities.push(entity_builder);
        entity
    }
    #[doc = "Applied the command buffer to the component store clearing the buffer afterwards"]
    pub fn build(&mut self, store: &mut MyEcsComponentStore<'position>) {
        self.deleted_entities.drain().for_each(|entity| {
            store.kill(entity);
        });
        self.new_entities
            .drain(..)
            .for_each(|builder| store.build(builder));
        for (entity, value) in self.add_position.drain() {
            if store.alive(entity) {
                store.add_position(entity, value);
            }
        }
        for entity in self.del_position.drain() {
            if store.alive(entity) {
                store.del_position(entity);
            }
        }
        for (entity, value) in self.add_velocity.drain() {
            if store.alive(entity) {
                store.add_velocity(entity, value);
            }
        }
        for entity in self.del_velocity.drain() {
            if store.alive(entity) {
                store.del_velocity(entity);
            }
        }
        for (entity, value) in self.add_acceleration.drain() {
            if store.alive(entity) {
                store.add_acceleration(entity, value);
            }
        }
        for entity in self.del_acceleration.drain() {
            if store.alive(entity) {
                store.del_acceleration(entity);
            }
        }
        for (entity, value) in self.add_enabled.drain() {
            if store.alive(entity) {
                store.add_enabled(entity, value);
            }
        }
        for entity in self.del_enabled.drain() {
            if store.alive(entity) {
                store.del_enabled(entity);
            }
        }
    }
    #[doc = "Schedules the deletion of an entity"]
    pub fn delete(&mut self, entity: ::secs::Entity) -> &mut Self {
        self.deleted_entities.insert(entity);
        self
    }
    #[doc = "Schedule the addition of the component 'position' of type [`crate::Position<'position>`] to the `entity`"]
    pub fn position(
        &mut self,
        entity: ::secs::Entity,
        value: crate::Position<'position>,
    ) -> &mut Self {
        self.add_position.insert(entity, value);
        self
    }
    #[doc = "Schedule the removal of the component 'position' of type [`crate::Position<'position>`] to the `entity`"]
    pub fn del_position(&mut self, entity: ::secs::Entity) -> &mut Self {
        self.del_position.insert(entity);
        self
    }
    #[doc = "Schedule the addition of the component 'velocity' of type [`crate::Velocity`] to the `entity`"]
    pub fn velocity(&mut self, entity: ::secs::Entity, value: crate::Velocity) -> &mut Self {
        self.add_velocity.insert(entity, value);
        self
    }
    #[doc = "Schedule the removal of the component 'velocity' of type [`crate::Velocity`] to the `entity`"]
    pub fn del_velocity(&mut self, entity: ::secs::Entity) -> &mut Self {
        self.del_velocity.insert(entity);
        self
    }
    #[doc = "Schedule the addition of the component 'acceleration' of type [`crate::Acceleration`] to the `entity`"]
    pub fn acceleration(
        &mut self,
        entity: ::secs::Entity,
        value: crate::Acceleration,
    ) -> &mut Self {
        self.add_acceleration.insert(entity, value);
        self
    }
    #[doc = "Schedule the removal of the component 'acceleration' of type [`crate::Acceleration`] to the `entity`"]
    pub fn del_acceleration(&mut self, entity: ::secs::Entity) -> &mut Self {
        self.del_acceleration.insert(entity);
        self
    }
    #[doc = "Schedule the addition of the component 'enabled' of type [`crate::Enabled`] to the `entity`"]
    pub fn enabled(&mut self, entity: ::secs::Entity, value: crate::Enabled) -> &mut Self {
        self.add_enabled.insert(entity, value);
        self
    }
    #[doc = "Schedule the removal of the component 'enabled' of type [`crate::Enabled`] to the `entity`"]
    pub fn del_enabled(&mut self, entity: ::secs::Entity) -> &mut Self {
        self.del_enabled.insert(entity);
        self
    }
}
