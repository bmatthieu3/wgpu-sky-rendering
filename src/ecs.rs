#![allow(non_snake_case)]

#[derive(Clone, Copy)]
#[derive(PartialEq, Eq)]
#[derive(Debug)]
pub struct Entity(pub usize);

use crate::shared::Shared;
impl Entity {
    pub fn new(world: &mut World) -> Self {
        let entity = if world.free_entities.is_empty() {
            let entity = Entity(world.num_entities);
            world.num_entities += 1;
    
            for c in &mut *world.components {
                c.push_none();
            }

            entity
        } else {
            let entity = world.free_entities.pop().unwrap();
            entity
        };

        entity
    }

    pub fn add<'a, T>(self, component: T, world: &'a mut World) -> Self
    where
        T: Component<'a> + 'static
    {
        // Loop over the type of components'vec to see if there is one matching
        for c in world.components.iter_mut() {
            if let Some(c) = c.as_any_mut()
                .downcast_mut::<Vec<Option<T>>>()
            {
                c[self.0] = Some(component);
                return self;
            }
        }

        // The component type has not been found
        // create a new one
        let mut component_vec = Vec::with_capacity(world.num_entities);
        for _ in 0..world.num_entities {
            component_vec.push_none();
        }
        component_vec[self.0] = Some(component);
        // Add it to the list of component
        world.components.push(Box::new(component_vec));

        self
    }

    pub fn remove<'a, T>(self, world: &'a mut World) -> Self
    where
        T: Component<'a> + 'static
    {
        // Loop over the type of components'vec to see if there is one matching
        for (idx, c) in world.components.iter_mut().enumerate() {
            if let Some(c) = c.as_any_mut()
                .downcast_mut::<Vec<Option<T>>>()
            {
                c.set_none(self.0);
                let used_component = c.used_component();

                if !used_component {
                    world.components.remove(idx);
                }
                return self;
            }
        }

        // The component type has not been found
        // Then we do nothing
        self
    }

    pub fn get<'a, T>(&self, world: &'a World) -> Option<&'a T>
    where
        T: Component<'a> + 'static
    {
        for component_vec in world.components.iter() {
            if let Some(c) = component_vec.as_any()
                .downcast_ref::<Vec<Option<T>>>()
            {
                return c[self.0].as_ref();
            }
        }

        None
    }

    pub fn get_mut<'a, T>(&self, mut world: &'a mut World) -> Option<&'a mut T>
    where
        T: Component<'a> + 'static
    {
        for component_vec in world.components.iter_mut() {
            if let Some(c) = component_vec.as_any_mut()
                .downcast_mut::<Vec<Option<T>>>()
            {
                return c[self.0].as_mut();
            }
        }

        None
    }

    pub fn set<'a, T>(&self, world: &'a mut World, v: T)
    where
        T: Component<'a> + 'static
    {
        self.add(v, world);
    }
}

pub trait Component<'a> {
    type RefType: 'a;
    type RefMutType: 'a;

    fn query(world: &'a World) -> Box<dyn Iterator<Item=Self::RefType> + 'a>;
    fn query_with_entity(world: &'a World) -> Box<dyn Iterator<Item=(Entity, Self::RefType)> + 'a>;

    fn query_mut(world: &'a mut World) -> Box<dyn Iterator<Item=Self::RefMutType> + 'a>;
}

pub trait ComponentVec {
    fn push_none(&mut self);
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn as_any(&self) -> &dyn std::any::Any;
    fn set_none(&mut self, idx: usize);

    fn used_component(&self) -> bool;
}

impl<'a, T> ComponentVec for Vec<Option<T>>
where
    T: Component<'a> + 'static
{
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn push_none(&mut self) {
        self.push(None);
    }

    fn set_none(&mut self, idx: usize) {
        self[idx] = None;
    }

    fn used_component(&self) -> bool {
        let used_component = self.iter()
            .any(|c| c.is_some());
        used_component
    }
}
use std::{fmt::Debug, time};

pub struct World {
    pub components: Vec<Box<dyn ComponentVec>>,
    num_entities: usize, // includes the free entities

    free_entities: Vec<Entity>,
}

pub struct SystemManager {
    systems: Vec<Box<dyn System>>,
    t: time::Instant,
}

impl SystemManager {
    pub fn new() -> Self {
        let t = time::Instant::now();
        Self {
            systems: vec![],
            t
        }
    }

    pub fn register_system<S>(&mut self, system: S) -> &mut Self
    where
        S: System + 'static
    {
        self.systems.push(Box::new(system));

        self
    }
    
    pub fn run(&self, game: &mut Game) {
        for system in &self.systems {
            system.run(game, &self.t);
        }
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            components: vec![],
            num_entities: 0,
            free_entities: vec![],
        }
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        self.free_entities.push(entity);

        // erase all the components from that entity
        for c in self.components.iter_mut() {
            c.set_none(entity.0);
        }
        
        self.components = self.components.drain(..)
            .filter(|c| {
                let used_component = c.used_component();
                used_component
            })
            .collect::<Vec<_>>();
    }

    pub fn query<'a, T>(&'a self) -> Box<dyn Iterator<Item=T::RefType> + 'a>
    where
        T: Component<'a>
    {
        T::query(self)
    }

    pub fn query_with_entity<'a, T>(&'a self) -> Box<dyn Iterator<Item=(Entity, T::RefType)> + 'a>
    where
        T: Component<'a>,
    {
        T::query_with_entity(self)
    }

    pub fn query_mut<'a, T>(&'a mut self) -> Box<dyn Iterator<Item=T::RefMutType> + 'a>
    where
        T: Component<'a>
    {
        T::query_mut(self)
    }

    pub fn contains<T>(&self) -> Option<usize>
    where
        T: Sized + 'static
    {
        let borrowed_c = &self.components;

        let num_c_types = borrowed_c.len();
        for c_idx in 0..num_c_types {
            if let Some(_) = borrowed_c[c_idx].as_any()
                .downcast_ref::<Vec<Option<T>>>()
            {
                return Some(c_idx);
            }
        }

        None
    }

    pub fn get<T>(&self) -> std::slice::Iter<'_, Option<T>>
    where
        T: Sized + 'static,
    {
        for component_vec in self.components.iter() {
            if let Some(c) = component_vec.as_any()
                .downcast_ref::<Vec<Option<T>>>()
            {
                return c.iter();
            }
        }

        [].iter()
    }

    unsafe fn get_mut<T>(&mut self) -> Option<*mut Box<dyn ComponentVec>>
    where
        T: Sized + 'static,
    {
        if let Some(idx) = self.get_index::<T>() {
            let ptr = self.components
                .as_mut_ptr()
                .offset(idx as isize) as *mut Box<dyn ComponentVec>;

            Some(ptr)
        } else {
            None
        }
    }

    pub fn get_index<T>(&self) -> Option<usize>
    where
        T: Sized + 'static,
    {
        for (idx, c) in self.components.iter().enumerate() {
            if let Some(_) = c.as_any()
                .downcast_ref::<Vec<Option<T>>>()
            {
                return Some(idx);
            }
        }

        None
    }
}

use crate::world::Game;
// A system that will update the position of all the entities
// having physics components
pub trait System {
    fn run(&self, game: &mut Game, t: &time::Instant);
}

use itertools::izip;

macro_rules! tuple_impls {
    ( $t1:ident, $( $t2:ident ),+ ) => {
        impl<'a, $t1, $($t2),+> Component<'a> for ($t1, $($t2),+ )
        where
            $t1: Component<'a> + 'static,
            $($t2: Component<'a> + 'static,)+
        {
            type RefType = (&'a $t1, $(&'a $t2),+ );
            type RefMutType = (&'a mut $t1, $(&'a mut $t2),+ );

            fn query(world: &'a World) -> Box<dyn Iterator<Item=Self::RefType> + 'a> {
                let $t1 = world.get::<$t1>();
                let ($($t2),+) = ($( world.get::<$t2>() ),+);

                Box::new(
                    izip!(
                        $t1,
                        $($t2),+
                    ).filter_map(|( $t1, $($t2),+ )| {
                        Some( ( $t1.as_ref()?, $($t2.as_ref()?),+ ) )
                    })
                )
            }

            fn query_with_entity(world: &'a World) -> Box<dyn Iterator<Item=(Entity, Self::RefType)> + 'a> {
                let $t1 = world.get::<$t1>();
                let ($($t2),+) = ($( world.get::<$t2>() ),+);

                Box::new(
                    izip!(
                        $t1,
                        $($t2),+
                    )
                    .enumerate()
                    .filter_map(|(entity, ( $t1, $($t2),+ ))| {
                        Some( (Entity(entity), ( $t1.as_ref()?, $($t2.as_ref()?),+ )) )
                    })
                )
            }

            fn query_mut(world: &'a mut World) -> Box<dyn Iterator<Item=Self::RefMutType> + 'a> {
                let $t1 = unsafe {
                    if let Some(c) = world.get_mut::<$t1>() {
                        (&mut *c).as_any_mut()
                            .downcast_mut::<Vec<Option<$t1>>>()
                            .unwrap()
                    } else {
                        return Box::new(std::iter::empty());
                    }
                };

                $(
                    let $t2 = unsafe {
                        if let Some(c) = world.get_mut::<$t2>() {
                            (&mut *c).as_any_mut()
                                .downcast_mut::<Vec<Option<$t2>>>()
                                .unwrap()
                        } else {
                            return Box::new(std::iter::empty());
                        }
                    };
                )+

                Box::new(
                    izip!($t1, $($t2),+)
                        .filter_map(|( $t1, $($t2),+ )| {
                            Some( ( $t1.as_mut()?, $($t2.as_mut()?),+ ) )
                        })
                )
            }
        }
    };
}

tuple_impls! { A, B }
tuple_impls! { A, B, C }
tuple_impls! { A, B, C, D }
tuple_impls! { A, B, C, D, E }
tuple_impls! { A, B, C, D, E, F }
tuple_impls! { A, B, C, D, E, F, G }
tuple_impls! { A, B, C, D, E, F, G, H }


mod tests {
    use crate::core_engine::Component;
    use crate::ecs;
    #[derive(Component)]
    #[derive(Debug)]
    #[derive(PartialEq)]
    struct Position(f32);

    #[derive(Component)]
    #[derive(Debug)]
    #[derive(PartialEq)]
    struct Velocity(f32);

    #[test]
    fn query_position() {
        use super::{World, Entity};
        let mut world = World::new();

        let _ = Entity::new(&mut world)
            .add(Position(0.0), &mut world)
            .add(Velocity(0.0), &mut world);

        let pos_vel = world.query::<(Position, Velocity)>().collect::<Vec<_>>();
        assert_eq!(pos_vel.len(), 1);
    }

    #[test]
    fn add_and_remove_component() {
        use super::{World, Entity};
        let mut world = World::new();

        let entity = Entity::new(&mut world)
            .add(Position(0.0), &mut world)
            .add(Velocity(0.0), &mut world);

        entity.remove::<Velocity>(&mut world);

        assert_eq!(world.num_entities, 1);
        assert_eq!(world.components.len(), 1);
    }

    #[test]
    fn add_multiple_entities() {
        use super::{World, Entity};

        let mut world = World::new();

        let _ = Entity::new(&mut world)
            .add(Position(1.0), &mut world);

        let _ = Entity::new(&mut world)
            .add(Position(2.0), &mut world, );

        assert_eq!(world.num_entities, 2);
        assert_eq!(world.components.len(), 1);
    }

    #[test]
    fn query_mut_one_component() {
        use super::{World, Entity};

        let mut world = World::new();

        let _ = Entity::new(&mut world)
            .add(Position(1.0), &mut world, );

        let _ = Entity::new(&mut world);

        for pos in world.query_mut::<Position>() {
            pos.0 += 1.0;
        }

        let pos = world.query::<Position>().collect::<Vec<_>>();
        assert_eq!(pos, vec![&Position(2.0)]);
    }

    #[test]
    fn query_mut_two_components() {
        use super::{World, Entity};

        let mut world = World::new();

        let _ = Entity::new(&mut world)
            .add(Position(1.0), &mut world, )
            .add(Velocity(2.0), &mut world, );

        let _ = Entity::new(&mut world)
            .add(Position(2.0), &mut world, );

        for (p, v) in world.query_mut::<(Position, Velocity)>() {
            p.0 += 1.0;
            v.0 += 1.0;
        }

        let pv = world.query::<(Position, Velocity)>().collect::<Vec<_>>();
        assert_eq!(pv, vec![(&Position(2.0), &Velocity(3.0))]);
    }

    #[test]
    fn query_mut_two_components_bis() {
        use super::{World, Entity};

        let mut world = World::new();

        let _ = Entity::new(&mut world)
            .add(Position(1.0), &mut world, );

        let _ = Entity::new(&mut world)
            .add(Position(2.0), &mut world, );

        for (p, v) in world.query_mut::<(Position, Velocity)>() {
            p.0 += 1.0;
            v.0 += 1.0;
        }

        let pv = world.query::<(Position, Velocity)>().collect::<Vec<_>>();
        assert_eq!(pv, vec![]);
    }

    #[test]
    fn query_mut_one_component_multiple_times() {
        use super::{World, Entity};

        let mut world = World::new();

        let _ = Entity::new(&mut world)
            .add(Position(1.0), &mut world, );

        let _ = Entity::new(&mut world)
            .add(Position(2.0), &mut world, );

        for (p1, p2) in world.query_mut::<(Position, Position)>() {
            p1.0 += 1.0;
            p2.0 += 1.0;
        }

        let pv = world.query::<Position>().collect::<Vec<_>>();
        assert_eq!(pv, vec![&Position(3.0), &Position(4.0)]);
    }

    #[test]
    fn add_and_remove_entity() {
        use super::{World, Entity};

        let mut world = World::new();

        let _ = Entity::new(&mut world)
            .add(Position(1.0), &mut world, );

        let e2 = Entity::new(&mut world)
            .add(Velocity(2.0), &mut world, );

        world.remove_entity(e2);

        assert_eq!(world.num_entities, 2);
        assert_eq!(world.free_entities.len(), 1);
        assert_eq!(world.free_entities[0], e2);

        // check for 1 component
        assert_eq!(world.components.len(), 1);

        assert!(world.components[0].as_any().downcast_ref::<Vec<Option<Position>>>().is_some());

        let positions = world.query::<Position>().collect::<Vec<_>>();
        assert_eq!(positions.len(), 1);
        assert_eq!(*positions[0], Position(1.0));
    }

    #[test]
    fn test_entities() {
        use super::{World, Entity};

        let mut world = World::new();

        let _ = Entity::new(&mut world)
            .add(Position(1.0), &mut world, );

        let e2 = Entity::new(&mut world)
            .add(Velocity(2.0), &mut world, );

        world.remove_entity(e2);

        let _ = Entity::new(&mut world)
            .add(Position(3.0), &mut world, );

        assert_eq!(world.num_entities, 2);
        assert_eq!(world.free_entities.len(), 0);

        // check for 1 component
        assert_eq!(world.components.len(), 1);

        assert!(world.components[0].as_any().downcast_ref::<Vec<Option<Position>>>().is_some());

        let pos = world.query::<Position>().collect::<Vec<_>>();
        assert_eq!(pos.len(), 2);

        let expected_pos = vec![
            Position(1.0),
            Position(3.0)
        ];
        for (pos, expected_pos) in pos.into_iter().zip(expected_pos.iter()) {
            assert_eq!(*pos, *expected_pos);
        }
    }
}