#![allow(unused_variables)]
#[derive(Clone, Debug)]
pub struct MyEcs {
    index_: usize,
    alive: ::secs::hibitset::BitSet,
    comp_position: Vec<Option<crate::Position>>,
    comp_velocity: Vec<Option<crate::Velocity>>,
    comp_acceleration: Vec<Option<crate::Acceleration>>,
    comp_enabled: (),
    com_bitset_position: ::secs::hibitset::BitSet,
    com_bitset_velocity: ::secs::hibitset::BitSet,
    com_bitset_acceleration: ::secs::hibitset::BitSet,
    com_bitset_enabled: ::secs::hibitset::BitSet,
    resource_delta_time: crate::DeltaTime,
}
impl MyEcs {
    #[doc = "Creates a builder for this ECS"]
    pub fn builder() -> MyEcsBuilder {
        MyEcsBuilder::new()
    }
    #[doc = "Runs the ECS"]
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for id in ::secs::hibitset::BitSetAnd(
            ::secs::hibitset::BitSetAnd(
                &self.com_bitset_enabled,
                ::secs::hibitset::BitSetAnd(&self.com_bitset_velocity, &self.com_bitset_position),
            ),
            &self.alive,
        ) {
            let id = ::secs::Entity::new(id);
            let sys_physics_comp_position = self
                .comp_position
                .get_mut(id.index() as usize)
                .unwrap()
                .as_mut()
                .unwrap();
            let sys_physics_comp_velocity = self
                .comp_velocity
                .get(id.index() as usize)
                .unwrap()
                .as_ref()
                .unwrap();
            let sys_physics_comp_enabled = if self.com_bitset_enabled.contains(id.index()) {
                Some(Default::default())
            } else {
                None
            }
            .unwrap();
            crate::physics_system(
                sys_physics_comp_position,
                sys_physics_comp_velocity,
                sys_physics_comp_enabled,
            );
        }
        Ok(())
    }
    #[doc = "Creates a new entity"]
    pub fn push(&mut self) -> ::secs::Entity {
        let entity = ::secs::Entity::new(self.index_ as u32);
        self.alive.add(self.index_ as u32);
        self.index_ += 1;
        self.comp_position.push(None);
        self.comp_velocity.push(None);
        self.comp_acceleration.push(None);
        entity
    }
    #[doc = "Deletes an entity, returns true if this entity was alive"]
    pub fn delete(&mut self, entity: ::secs::Entity) -> bool {
        if self.alive.remove(entity.index()) {
            {
                let exists = self.com_bitset_position.remove(entity.index());
                self.comp_position[entity.index() as usize].take();
            }
            {
                let exists = self.com_bitset_velocity.remove(entity.index());
                self.comp_velocity[entity.index() as usize].take();
            }
            {
                let exists = self.com_bitset_acceleration.remove(entity.index());
                self.comp_acceleration[entity.index() as usize].take();
            }
            {
                let exists = self.com_bitset_enabled.remove(entity.index());
                self.comp_enabled;
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
    #[doc = "Adds the component 'position' of type [`crate::Position`] to the `entity`"]
    pub fn comp_position(&mut self, entity: ::secs::Entity, value: crate::Position) -> &mut Self {
        assert!(
            (entity.index() as usize) < self.index_,
            "Entity ID is not in the existing range"
        );
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        self.com_bitset_position.add(entity.index());
        self.comp_position[entity.index() as usize] = Some(value);
        self
    }
    #[doc = "Removes the component 'position' of type [`crate::Position`] from the `entity`, returns the component if it had it"]
    pub fn remove_comp_position(&mut self, entity: ::secs::Entity) -> Option<crate::Position> {
        assert!(
            (entity.index() as usize) < self.index_,
            "Entity ID is not in the existing range"
        );
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        let exists = self.com_bitset_position.remove(entity.index());
        self.comp_position[entity.index() as usize].take()
    }
    #[doc = "Adds the component 'velocity' of type [`crate::Velocity`] to the `entity`"]
    pub fn comp_velocity(&mut self, entity: ::secs::Entity, value: crate::Velocity) -> &mut Self {
        assert!(
            (entity.index() as usize) < self.index_,
            "Entity ID is not in the existing range"
        );
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        self.com_bitset_velocity.add(entity.index());
        self.comp_velocity[entity.index() as usize] = Some(value);
        self
    }
    #[doc = "Removes the component 'velocity' of type [`crate::Velocity`] from the `entity`, returns the component if it had it"]
    pub fn remove_comp_velocity(&mut self, entity: ::secs::Entity) -> Option<crate::Velocity> {
        assert!(
            (entity.index() as usize) < self.index_,
            "Entity ID is not in the existing range"
        );
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        let exists = self.com_bitset_velocity.remove(entity.index());
        self.comp_velocity[entity.index() as usize].take()
    }
    #[doc = "Adds the component 'acceleration' of type [`crate::Acceleration`] to the `entity`"]
    pub fn comp_acceleration(
        &mut self,
        entity: ::secs::Entity,
        value: crate::Acceleration,
    ) -> &mut Self {
        assert!(
            (entity.index() as usize) < self.index_,
            "Entity ID is not in the existing range"
        );
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        self.com_bitset_acceleration.add(entity.index());
        self.comp_acceleration[entity.index() as usize] = Some(value);
        self
    }
    #[doc = "Removes the component 'acceleration' of type [`crate::Acceleration`] from the `entity`, returns the component if it had it"]
    pub fn remove_comp_acceleration(
        &mut self,
        entity: ::secs::Entity,
    ) -> Option<crate::Acceleration> {
        assert!(
            (entity.index() as usize) < self.index_,
            "Entity ID is not in the existing range"
        );
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        let exists = self.com_bitset_acceleration.remove(entity.index());
        self.comp_acceleration[entity.index() as usize].take()
    }
    #[doc = "Adds the component 'enabled' of type [`crate::Enabled`] to the `entity`"]
    pub fn comp_enabled(&mut self, entity: ::secs::Entity, value: crate::Enabled) -> &mut Self {
        assert!(
            (entity.index() as usize) < self.index_,
            "Entity ID is not in the existing range"
        );
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        self.com_bitset_enabled.add(entity.index());
        self.comp_enabled;
        self
    }
    #[doc = "Removes the component 'enabled' of type [`crate::Enabled`] from the `entity`, returns the component if it had it"]
    pub fn remove_comp_enabled(&mut self, entity: ::secs::Entity) -> Option<crate::Enabled> {
        assert!(
            (entity.index() as usize) < self.index_,
            "Entity ID is not in the existing range"
        );
        assert!(self.alive.contains(entity.index()), "Entity is not alive");
        let exists = self.com_bitset_enabled.remove(entity.index());
        self.comp_enabled;
        if exists {
            Some(crate::Enabled::default())
        } else {
            None
        }
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
            alive: ::secs::hibitset::BitSet::new(),
            index_: 0,
            resource_delta_time: self.resource_delta_time,
            comp_position: Vec::new(),
            comp_velocity: Vec::new(),
            comp_acceleration: Vec::new(),
            comp_enabled: (),
            com_bitset_position: ::secs::hibitset::BitSet::new(),
            com_bitset_velocity: ::secs::hibitset::BitSet::new(),
            com_bitset_acceleration: ::secs::hibitset::BitSet::new(),
            com_bitset_enabled: ::secs::hibitset::BitSet::new(),
        }
    }
    #[doc = "Builds the builder into the ECS with a capacity"]
    pub fn with_capacity(self, capacity: usize) -> MyEcs {
        MyEcs {
            alive: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
            index_: 0,
            resource_delta_time: self.resource_delta_time,
            comp_position: Vec::with_capacity(capacity),
            comp_velocity: Vec::with_capacity(capacity),
            comp_acceleration: Vec::with_capacity(capacity),
            comp_enabled: (),
            com_bitset_position: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
            com_bitset_velocity: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
            com_bitset_acceleration: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
            com_bitset_enabled: ::secs::hibitset::BitSet::with_capacity(capacity as u32),
        }
    }
    #[doc = "Sets the resource 'delta_time' of type [`crate::DeltaTime`]"]
    pub fn resource_delta_time(mut self, value: crate::DeltaTime) -> Self {
        self.resource_delta_time = value;
        self
    }
}
