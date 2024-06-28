use bevy::{
    ecs::entity::EntityHashMap, prelude::*, render::render_resource::ComputePass,
    utils::hashbrown::hash_map::Entry, utils::TypeIdMap,
};
use std::{
    any::TypeId,
    sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ComputeFunctionId(u32);

pub trait ComputeFunction<I: ComputePhaseItem>: Send + Sync + 'static {
    #[allow(unused_variables)]
    fn prepare(&mut self, world: &'_ World) {}

    fn compute<'w>(&mut self, world: &'w World, pass: &mut ComputePass<'w>, view: Entity, item: &I);
}

pub trait ComputePhaseItem: Sized + Send + Sync + 'static {
    fn entity(&self) -> Entity;

    fn compute_function(&self) -> ComputeFunctionId;
}

#[derive(Resource, Deref, DerefMut)]
pub struct ViewComputePhases<I: ComputePhaseItem>(pub EntityHashMap<ComputePhase<I>>);

impl<I: ComputePhaseItem> Default for ViewComputePhases<I> {
    fn default() -> Self {
        Self(default())
    }
}

impl<I: ComputePhaseItem> ViewComputePhases<I> {
    pub fn insert_or_clear(&mut self, entity: Entity) {
        match self.entry(entity) {
            Entry::Occupied(mut entry) => entry.get_mut().clear(),
            Entry::Vacant(entry) => {
                entry.insert(default());
            }
        }
    }
}

pub struct ComputePhase<I: ComputePhaseItem> {
    pub items: Vec<I>,
}

impl<I: ComputePhaseItem> Default for ComputePhase<I> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<I: ComputePhaseItem> ComputePhase<I> {
    #[inline]
    pub fn add(&mut self, item: I) {
        self.items.push(item);
    }

    pub fn compute<'w>(&self, compute_pass: &mut ComputePass<'w>, world: &'w World, view: Entity) {
        let compute_functions = world.resource::<ComputeFunctions<I>>();
        let mut compute_functions = compute_functions.write();
        compute_functions.prepare(world);

        for item in self.items.iter() {
            let Some(compute_function) = compute_functions.get_mut(item.compute_function()) else {
                continue;
            };

            compute_function.compute(world, compute_pass, view, item);
        }
    }

    pub fn clear(&mut self) {
        self.items.clear()
    }
}

pub struct ComputeFunctionsInternal<I: ComputePhaseItem> {
    pub compute_functions: Vec<Box<dyn ComputeFunction<I>>>,
    pub indices: TypeIdMap<ComputeFunctionId>,
}

impl<I: ComputePhaseItem> ComputeFunctionsInternal<I> {
    pub fn prepare(&mut self, world: &World) {
        for function in &mut self.compute_functions {
            function.prepare(world);
        }
    }

    pub fn add<F: ComputeFunction<I>>(&mut self, compute_function: F) -> ComputeFunctionId {
        self.add_with::<F, F>(compute_function)
    }

    pub fn add_with<T: 'static, F: ComputeFunction<I>>(
        &mut self,
        compute_function: F,
    ) -> ComputeFunctionId {
        let id = ComputeFunctionId(self.compute_functions.len().try_into().unwrap());
        self.compute_functions.push(Box::new(compute_function));
        self.indices.insert(TypeId::of::<T>(), id);
        id
    }

    pub fn get_mut(&mut self, id: ComputeFunctionId) -> Option<&mut dyn ComputeFunction<I>> {
        self.compute_functions
            .get_mut(id.0 as usize)
            .map(|f| &mut **f)
    }

    pub fn get_id<T: 'static>(&self) -> Option<ComputeFunctionId> {
        self.indices.get(&TypeId::of::<T>()).copied()
    }

    pub fn id<T: 'static>(&self) -> ComputeFunctionId {
        self.get_id::<T>().unwrap_or_else(|| {
            panic!(
                "Compute function {} not found for {}",
                std::any::type_name::<T>(),
                std::any::type_name::<I>()
            )
        })
    }
}

#[derive(Resource)]
pub struct ComputeFunctions<I: ComputePhaseItem> {
    internal: RwLock<ComputeFunctionsInternal<I>>,
}

impl<I: ComputePhaseItem> Default for ComputeFunctions<I> {
    fn default() -> Self {
        Self {
            internal: RwLock::new(ComputeFunctionsInternal {
                compute_functions: Vec::new(),
                indices: Default::default(),
            }),
        }
    }
}

impl<I: ComputePhaseItem> ComputeFunctions<I> {
    pub fn read(&self) -> RwLockReadGuard<'_, ComputeFunctionsInternal<I>> {
        self.internal.read().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, ComputeFunctionsInternal<I>> {
        self.internal
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

pub trait AddComputeFunction {
    fn add_compute_function<I: ComputePhaseItem, F: ComputeFunction<I> + FromWorld>(
        &mut self,
    ) -> &mut Self;
}

impl AddComputeFunction for SubApp {
    fn add_compute_function<I: ComputePhaseItem, F: ComputeFunction<I> + FromWorld>(
        &mut self,
    ) -> &mut Self {
        let compute_function = F::from_world(self.world_mut());
        let compute_functions = self
            .world()
            .get_resource::<ComputeFunctions<I>>()
            .unwrap_or_else(|| {
                panic!(
                    "ComputeFunctions<{}> must be added to the world as a resource \
                     before adding compute functions to it",
                    std::any::type_name::<I>(),
                );
            });
        compute_functions.write().add_with::<F, _>(compute_function);
        self
    }
}

impl AddComputeFunction for App {
    fn add_compute_function<I: ComputePhaseItem, F: ComputeFunction<I> + FromWorld>(
        &mut self,
    ) -> &mut Self {
        SubApp::add_compute_function::<I, F>(self.main_mut());
        self
    }
}
