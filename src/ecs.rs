#[derive(Clone, Copy)]
struct Entity(usize);

trait Component<'a> {    
    fn query(world: &'a Game) -> Box<dyn Iterator<Item=Self>> where Self: 'a;
}

use std::vec;

use futures::stream::futures_unordered::Iter;

use crate::math::Vec3;
pub struct Position {
    velocity: Vec3<f32>,
    position: Vec3<f32>,
}

fn get<'a, T>(world: &'a Game) -> Box<dyn Iterator<Item=&'a Option<T>> + 'a>
    where
        T: Sized + 'static
{
    for component_vec in &world.components {
        if let Some(component_vec) = component_vec.as_any()
            .downcast_ref::<Vec<Option<T>>>()
        {
            let it = component_vec.iter();
            return Box::new(it);
        }
    }

    Box::new(std::iter::empty())
}
trait ComponentVec {
    fn push_none(&mut self);
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn as_any(&self) -> &dyn std::any::Any;
    fn set_none(&mut self, idx: usize);

    fn used_component(&self) -> bool;
}

impl<T> ComponentVec for Vec<Option<T>>
where T: Component + 'static {
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

struct Game {
    components: Vec<Box<dyn ComponentVec>>,
    num_entities: usize, // includes the free entities

    free_entities: Vec<Entity>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            components: vec![],
            num_entities: 0,
            free_entities: vec![],
        }
    }

    pub fn add_entity(&mut self) -> Entity {
        let entity = if self.free_entities.is_empty() {
            let entity = Entity(self.num_entities);
            self.num_entities += 1;
    
            for component_vec in &mut self.components {
                component_vec.push_none();
            }

            entity
        } else {
            let entity = self.free_entities.pop().unwrap();
            entity
        };

        entity
    }

    pub fn remove_entity(&mut self, entity: Entity) {
        self.free_entities.push(entity);

        // erase all the components from that entity
        for c in self.components.iter_mut() {
            c.set_none(entity.0);
        }

        // remove unused components
        self.components = self.components.drain(..)
            .filter(|c| {
                let used_component = c.used_component();
                used_component
            })
            .collect();
    }

    pub fn borrow_component_vec<'a, T>(&mut self) -> Option<&Vec<Option<T>>>
    where
        T: Component<'a> + 'static
    {
        // Loop over the type of components'vec to see if there is one matching
        for component_vec in &self.components {
            if let Some(component_vec) = component_vec.as_any()
                .downcast_ref::<Vec<Option<T>>>()
            {
                return Some(component_vec);
            }
        }

        return None;
    } 

    pub fn add_component_to_entity<'a, T>(&mut self, entity: Entity, component: T)
    where
        T: Component<'a> + 'static
    {
        // Loop over the type of components'vec to see if there is one matching
        for component_vec in &mut self.components {
            if let Some(component_vec) = component_vec.as_any_mut()
                .downcast_mut::<Vec<Option<T>>>()
            {
                component_vec[entity.0] = Some(component);
                return;
            }
        }

        // The component type has not been found
        // create a new one
        let mut component_vec = Vec::with_capacity(self.num_entities);
        for i in 0..self.num_entities {
            component_vec[i] = None;
        }
        component_vec[entity.0] = Some(component);
        // Add it to the list of component
        self.components.push(Box::new(component_vec));
    }

    pub fn remove_component_from_entity<'a, T>(&mut self, entity: Entity)
    where
        T: Component<'a> + 'static
    {
        // Loop over the type of components'vec to see if there is one matching
        for (idx, component_vec) in &mut self.components.iter_mut().enumerate() {
            if let Some(component_vec) = component_vec.as_any_mut()
                .downcast_mut::<Vec<Option<T>>>()
            {
                component_vec.set_none(entity.0);
                let used_component = component_vec.used_component();

                if !used_component {
                    self.components.remove(idx);
                }
                return;
            }
        }

        // The component type has not been found
        // Then we do nothing
    }
}

macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.zip(
            zip!($($y), +))
    )
}

macro_rules! tuple_impls {
    ( $y:ident ) => {
        impl<'a, $y: Component<'a> + 'static> Component<'a> for &'a $y
        {
            fn query(world: &'a Game) -> Box<dyn Iterator<Item=Self>> where Self: 'a {
                let it = if let Some(components) = world.borrow_component_vec::<$y>() {
                    components.iter()
                        .filter_map(|$y| Some($y.as_ref()?) )
                } else {
                    std::slice::iter::empty()
                };

                Box::new(it)
            }
        }
    };
    /*( $y:ident, $( $x:ident ),+ ) => {
        impl<'a, $y: Component + 'static, $($x: Component + 'static),+> Component for (&'a $y, $(&'a $x),+ )
        {
            fn query(world: &'a Game) -> Box<dyn Iterator<Item=Self>> where Self: 'a {
                let it = <($( &$x ),+)>::query(world);
                let it = get::<$y>(world).zip(it)
                    .filter_map(|($y, $( $x ),+)| {
                        Some( ($y.as_ref()?, $($x),+ ) )
                    });
            
                
                Box::new(it)
            }
        }
    };*/
}

fn query<'a, T>(world: &'a Game) -> Box<dyn Iterator<Item=T>>
where
    T: Component<'a> + 'static
{
    T::query(world)
}

tuple_impls! { A }
//tuple_impls! { A, B }